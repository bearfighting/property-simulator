# 架构与模块边界

> 本文保留原规范章节编号，便于和既有讨论对应。首版实施基线以本文及 [实施路线](implementation-roadmap.md) 为准。

## 2. 已确认决策摘要（正式 ADR 索引）

下表是当前基线的决策索引；正式记录见 [`docs/adr/`](adr/README.md)。

| ID | 决策 | 解释 |
|---|---|---|
| ADR-01 | Simulation 按月推进并接受外部运行控制 | 可用包含边界的起止月份、月末终止条件或逐月继续/停止控制；贷款结清、现金流阈值等只有被明确配置为条件时才结束模拟 |
| ADR-02 | 统一 **monthly tick** | 一月一个事务性结算周期；20 年 = 240 tick |
| ADR-03 | 从**任意有效初始快照**开始 | 不要求补齐模拟前的购房/还贷事件 |
| ADR-04 | Scenario Preset + 用户覆盖 | 初始事实与未来假设分离；每个 Slice 校验自己的输入 |
| ADR-05 | Slice 独立计算、统一提交 | Slice 不直接写共享 State/Ledger；输出 Proposals/Effects |
| ADR-06 | Event、Effect、Ledger 分离 | 事件说明发生了什么；Effect 改变状态；Ledger 记录经济事实 |
| ADR-07 | 现金流按月入账 | 保留每笔明细及其来源，月末产生聚合视图 |
| ADR-08 | 核心产物是 Dataset | JSON 完整交换，CSV/XLSX 扁平导出；不生成投资报告 |
| ADR-09 | 随机流显式管理；重新模拟可重新抽样 | 记录规则/参数与抽样配置；重新模拟创建新随机试验流，不承诺生成相同结果 |
| ADR-10 | 三阶段交付 | I 领域内核 M0–M5；II CLI 与持久化；III GPUI 桌面应用。各阶段顺序以实施路线为准；交互式领域决策不属于 v1。 |
| ADR-11 | 初期 Rust 静态注册 | 暂不做动态插件、通用配置语言或异步服务架构 |
| ADR-12 | v1 随机范畴独立 | 自然环境、房屋/组件、政策/宏观、居住/运营范畴间按独立处理；Slice 可读取明确生成的共享状态，但不暗中引入跨范畴随机相关。未来相关性须作为显式、版本化模型设计。 |
| [ADR-13](adr/ADR-13-ledger-profile.md) | 家庭房产 Ledger | 最小科目范围、期初净值及同月权责/现金简化 |
| [ADR-14](adr/ADR-14-mortgage-interest.md) | 贷款利息计算 | 月利率、付款顺序、资本化、封顶和舍入规则 |
| [ADR-15](adr/ADR-15-random-streams-and-branches.md) | 随机流、分支与恢复 | 新分支创建新 trial；checkpoint 恢复同一 trial |
| [ADR-16](adr/ADR-16-configuration-and-run-control.md) | 配置与运行控制 | 初始配置、月度输入、终止条件及历史分支 |
| [ADR-17](adr/ADR-17-money-and-identifiers.md) | 货币金额与标识符 | 单币种账本、不兑换；强类型 ID 与科目代码分离 |
| [ADR-18](adr/ADR-18-sufficient-state-transition.md) | 充分状态与单期转移 | 完整期初状态、当期输入/随机输入决定当期结果；历史仅作显式状态或审计记录 |
| [ADR-19](adr/ADR-19-validation-only-domain-errors.md) | 验证边界与计算错误 | 可纠正的结构错误在验证边界产生；运行期 fatal 携带阶段、月份、可用身份和原始错误 |
| [ADR-20](adr/ADR-20-composable-slices-and-random-sampling.md) | Slice 组合、来源标识与随机采样解耦 | 独立提案可交换合并；采样分布、采样器和 Slice 分离；不可恢复运行时错误终止运行 |


## 6. Slice 架构与契约

### 6.1 目标

领域计算的基本契约是“完整期初状态 + 当期冻结输入 + 当期随机输入 → 当期结果 + 期末状态”。Slice 是计算规则的代码边界，不是活动种类的固定容器：实现和活动实例不要求一一对应；一个实现可处理多个同类独立实例，也可由多个实现共同计算一项活动。每个生效活动经相应规则计算后产生带来源的事件、Effects 和 Ledger 交易。规则只依赖显式传入的状态、输入和同月中间结果；若需要历史信息，其继续有效的影响必须已进入明确定义的状态字段。事件与账簿历史可供审计、展示和投影，不作为隐式计算上下文。**Slice 不得直接修改共享世界或 Ledger。**

输入验证器负责拒绝并报告用户可修正的结构问题，并产生满足不变量的冻结输入。该边界之后，状态转移、余额重算和投影不得因缺失引用、分类或投影前置条件而失败。金额采用有限 `i128` 表示；运行时 checked overflow 是显式失败，不能包装成经济结果，也不能静默夹值。未预见的不变量破坏、实现缺陷或基础设施故障属于不可恢复的模拟失败，不是用户输入问题；运行停止并提供工程诊断，不承诺该故障之后可恢复或持久化一致。

单期协调器提供统一的状态转移：读取期初状态、本期冻结输入和显式采样值，按显式依赖计算 Slice，合并独立提案并生成期末状态。成功完成的月份必须满足状态、事件和账簿不变量；不可恢复运行时故障停止运行，但不承诺故障后的回滚或持久化一致性。协调器不读取总运行期限，也不选择下一期。更高层运行控制器决定是否再次调用单期协调器，并评估结束月份、终止条件和逐期继续/停止；快速运行与逐期交互复用同一个单期转移，不复制领域规则。

### 6.2 建议接口（概念草案）

```rust
trait SimulationSlice {
    fn id(&self) -> SliceId;
    fn version(&self) -> ModelVersion;
    fn required_inputs(&self) -> InputRequirements;
    fn dependencies(&self) -> Vec<SliceDependency>;
    fn validate(&self, scenario: &Scenario) -> Vec<ValidationIssue>;
    fn propose(&self, ctx: &SliceContext<'_>) -> Result<SliceProposal, SimulationError>;
}

struct SliceContext<'a> {
    period: YearMonth,
    opening_state: &'a WorldState,
    input: &'a FrozenMonthlyInput,
    prior_stage_results: &'a StageResults,
}

struct SliceProposal {
    events: Vec<ProposedEvent>,
    effects: Vec<ProposedEffect>,
    transactions: Vec<ProposedLedgerTransaction>,
    decisions: Vec<DecisionRequest>,
    scheduled_actions: Vec<ScheduledAction>,
}
```

Slice 只从上下文读取显式、类型化的本期采样值，不调用采样器。样本保留实例、Slice、模型版本和期间来源，交给 Slice 前由协调边界核对来源及样本类型；单期结果再次核对样本类型与活动实例。概率分布定义概率法则，采样器负责从分布产生具体值；二者与 Slice 分别实现，并在组合边界验证兼容性。采样流如何分配及 checkpoint 如何续接属于 M3。单期调用须能基于显式输入独立理解和测试。

该接口只是方向；初期可用静态 enum 和具体结构，避免过早动态分发、类型擦除及插件化。同月依赖使用有限阶段/有向依赖协议；Slice 不进行无界多轮互调。

`DecisionRequest` 表示需要策略处理的领域选择，不意味着暂停运行。v1 使用场景配置的自动规则解析；交互式等待用户输入属于未来能力。

### 6.3 共享资源与冲突

- 每个活动 Slice 实例按冻结输入生成强类型 `ResourceKey` 的 `reads`、`writes` 与实例依赖声明。依赖以有限 DAG 推导确定性阶段；阶段内按 `SliceId`、实例 ID 排序。缺失依赖、循环、来源与声明不符、未声明 Effect/凭证账户写入及无依赖路径的读写冲突在组合边界拒绝；处于不同拓扑阶段本身不构成依赖顺序。账簿科目资源键采用固定账户 ID 编码。
- 对无冲突的独立 Slice 提案，合并操作必须满足结合律和交换律，并提供空提案作为单位元；因此重排、分组或并行计算任意独立子集不改变合并结果。现金变化可按账户和分类加总；贷款本金、房屋状况、出租状态等状态 Effects 不得无条件相加。
- 每月结果由当月生效活动的已提交记录汇总；现金余额按分录推导，现金流按账户和分类投影。保留逐项活动及来源，不以净额替代活动记录。随机模型可决定活动是否发生及其数值，由活动规则生成对应结果。
- 多笔贷款由同一贷款规则实现逐笔计算并分别保存状态；首版贷款实例之间不建立相关性或联动规则。借款用途/资金流向由用户输入决定，不隐式假定贷款所得进入模拟现金账户或用于房产。
- Effect 显式声明 `ResourceKey` 目标；相同资源的相同状态值可幂等共存，异值写入拒绝。现金影响保留为独立、可追踪的复式凭证，由 Ledger 按账户推导余额，不在提案合并时净额化。
- 所有提案带 `slice_id`、Slice 实例 ID、`model_version`、`source_event_id`（若有）、`period`。
- Slice 实例由调用方赋予稳定 ID；协调器根据 trial/branch、月份、实例 ID、输出类型及稳定输出键派生事件、Effect 和流水 ID。ID 不依赖执行顺序、分组或 worker 调度。


## 13. Rust Workspace 与依赖方向

M0-B 已选定最小 workspace 边界，当前实现位于 `crates/`：

| Crate | 职责 | 依赖 |
|---|---|---|
| `property-simulation-domain` (`domain`) | 领域值、实体、不变量与纯领域规则 | 无 workspace crate 依赖 |
| `property-simulation-accounting` (`accounting`) | 科目、凭证、分录、余额和报表投影 | `domain` |
| `property-simulation-models` (`models`) | 领域模型策略与概率分布 | `domain` |
| `property-simulation-simulation` (`simulation`) | 月度生命周期、Slice 协调和运行控制 | `domain`、`accounting`、`models` |

Crate 内部保持小而明确的模块边界。`simulation::lib.rs` 只声明模块并重导出公开 API；`error` 统一模拟错误，`types` 保存共享状态、活动身份、提案数据与单期结果类型，`activity` 负责样本来源校验及 Slice 调用契约，`proposals` 负责稳定 ID 识别、来源链接和提案合并，`validation` 负责输入约束，`slices::rental` 与 `slices::repair` 分别实现示例活动规则。示例 Slice 依赖共享契约及领域/账务值，不由通用协调代码反向依赖具体 Slice。`accounting::transaction` 定义不可变凭证/分录及来源数据契约；`accounting::ledger` 持有账簿状态并负责交易验证、原子过账和余额推导，避免把依赖账簿上下文的不变量分散给独立服务。`models::lib.rs` 同样只聚合模块；`repair_cost` 管理维修成本支持范围，`sampling` 定义采样器接口。新增 Slice 应放在独立模块，并通过既有契约接入。

当前 M2.5 单期协调器位于 `simulation::coordinator`：它只协调已评估输出，不依赖或分派具体 Slice；Slice 输出在评估时绑定资源声明。协调器验证声明与来源后，在账簿副本上提交，并用显式 reducer 生成当前支持的期末状态。该模块依赖 simulation 契约及 accounting Ledger，不把业务转移逻辑放入 CLI/GUI 或账务层。

依赖图：

```text
accounting ─┐
            ├──> domain
models ─────┘

simulation ───> domain, accounting, models
```

这些 crate 目前只建立编译边界并记录各自职责，不提供空的占位 API；实际公开类型随对应里程碑加入。Infrastructure、CLI 和 GUI 暂不建 crate，待其有明确职责时再加入。Rust workspace 使用 edition 2024、stable toolchain 和集中配置；本地与 CI 共用 `scripts/check.sh`。

```text
property-simulation-sandbox/
├── Cargo.toml
├── crates/
│   ├── simulation-core/       # clock, state, event, effect, monthly runner
│   ├── simulation-models/     # distributions, hazards, random streams
│   ├── real-estate/           # property entities + domain slices
│   │   └── slices/
│   │       ├── acquisition_mortgage/
│   │       ├── tax_insurance/
│   │       ├── maintenance/
│   │       ├── deterioration_repair/
│   │       ├── rental/
│   │       ├── utilities/
│   │       └── disposal/
│   ├── simulation-ledger/     # accounts, transactions, validation
│   ├── simulation-storage/    # SQLite, checkpoints
│   ├── simulation-export/     # JSON, CSV, XLSX
│   ├── simulation-cli/        # batch runs and test fixtures
│   └── desktop/               # GPUI
├── scenarios/                 # versioned example presets
├── tests/                     # golden, property, statistical tests
└── docs/
    ├── project-spec.md
    ├── architecture/
    └── adr/
```

上述初始方案是候选结构参考。当前 M0-B crate 选择见本节开头；未来按真实编译边界扩展，保持 domain 不依赖 IO/展示层且不引入循环依赖。

**候选库**：`serde`、`serde_json`、`rand`/`rand_distr`、`rust_decimal`、`time`、`thiserror`、`tracing`、`clap`、`rusqlite` 或 `sqlx`、`csv`、`rust_xlsxwriter`、GPUI。具体版本与 Linux 平台兼容性在初始化时验证。


### 20.2 领域边界与建议类型

第一阶段按编译边界组织模块，不预先拆分成大量空 crate。建议的初始布局：

```text
crates/
  domain/          # YearMonth、金额、Scenario、WorldState、领域 ID 与错误
  accounting/      # 科目表、凭证、分录、余额、报表投影与校验
  simulation/      # 月度生命周期、Slice 契约、协调、Effects、随机流
  models/          # 利率、贷款摊还、分布、风险/退化模型
```

如早期实现证明单一 crate 更易演进，可先用一个 crate 的模块结构，保持依赖方向和公开接口清晰，再按实际编译/所有权边界拆分。不得为了目录结构提前引入循环依赖。

**基础类型与时间**

- `YearMonth`：有效年月、排序、加减月、包含端点的区间迭代；不使用日期天数表达月度模型。
- `Money`：带显式币种代码的十进制定点金额，禁止二进制浮点参与账务计算；一个账本首版只允许一种币种，不做兑换。货币符号和本地化格式由展示层处理。
- `RoundingPolicy`：中间计算使用十进制定点精度，最终入账金额按单币种货币最小单位四舍五入（正数 midpoint half-up）；舍入规则是模型版本的一部分。贷款利息按此规则舍入到分。若未来出现多币种或负数舍入场景，再补充相应约定。
- 强类型、不透明 ID：场景、运行、Slice、实体、事件、决策、凭证、账户及模型版本各用独立类型；`AccountCode` 是可读科目代码，不是 `AccountId`。ID 由领域调用方提供，核心计算不生成 ID；重试复用原凭证 ID。
- 所有可序列化模型有明确 schema/version；金额编码为带货币的十进制字符串。

**状态、历史与连续运行**

- `InitialWorldState` 表示起点事实，不自动创建对应的历史交易或事件。
- 初始账务输入由用户提供的所有已知资产、负债，以及用于生成未来收入/费用的假设构成；配置校验负责检查其是否足以构建期初账簿和所启用 Slice 的完整状态。不得静默补造余额或假设。
- `WorldState` 保存所有已知且会影响未来的领域变量，包括账户余额、以集合表达的多笔独立贷款及其条款/余额、组件状态/年龄、租赁状态、Slice 内部状态和共享环境状态。
- `SimulationHistory` 保存启动时的运行控制、用户在每月输入点提交的输入快照/变更、终止条件评估结果、已提交事件、Effects、会计凭证、月度快照及必要的运行元数据。收入、费用假设与实际过账金额应能区分来源。月末快照包含从该点继续所需的完整领域状态、账簿余额、Slice 状态和输入假设；新分支记录父运行 ID 与 fork 月份，沿用该快照前的历史作为共同祖先，后续生成记录属于新分支。
- `RuntimeCheckpoint` 保存未完成期间的待处理提案、执行阶段、待调度动作及随机流续跑所需状态。v1 不暂停等待交互式决策；若未来加入该能力，待决策状态再纳入 checkpoint。已结束月的重跑与未完成月的恢复分开定义。
- 输入的历史事实与模型生成的历史记录都进入可追溯历史；它们的来源类型不同，但均不能在续跑时丢失。
- 续跑边界必须验证充分状态：启用的 Slice 所需字段缺失时明确报错，不以隐式默认值补齐。

**事件与状态变化**

- `Event` 记录已发生事实；`Decision` 记录用户或策略选择；`Effect` 描述经过校验的状态变化；`LedgerTransaction` 记录该业务动作的会计影响。
- 记录携带稳定 ID、期间、来源 Slice/模型版本、因果引用和适用实体。
- Slice 只向只读上下文提交候选提案；统一协调器负责验证、冲突处理及成功期次的完整提交。不可恢复运行时故障后的回滚、恢复和持久化一致性不作保证。Slice 不直接改写世界状态或总账。
- 业务事件可以有多个 Effects 和多张有明确语义的凭证；导出/查询层不得把事件、状态变化和会计事实混成一条含义模糊的流水。
