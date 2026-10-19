# 架构与模块边界

> 本文保留原规范章节编号，便于和既有讨论对应。首版实施基线以本文及 [实施路线](implementation-roadmap.md) 为准。

## 2. 已确认决策摘要（正式 ADR 归档待 M0 完成）

下表是当前基线的决策索引，不代替 `docs/adr/` 中的正式 ADR。M0 将建立 ADR 模板并归档需要保留的决策及测试向量。

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


## 6. Slice 架构与契约

### 6.1 目标

每个 Slice 负责自己的领域状态、规则、事件类型、配置校验与测试；引擎负责时间、排序、依赖协调、随机流、提交、保存和输出。**Slice 不得直接修改共享世界或 Ledger。**

运行器提供统一的逐月执行流程：每月读取冻结输入，执行各 Slice，原子提交并生成月末快照，再评估运行控制。结束月份、终止条件和逐月继续/停止都是运行控制输入；快速运行与逐月交互由应用层决定何时提供这些输入，不得复制领域规则或实现两套模拟流程。

### 6.2 建议接口（概念草案）

```rust
trait SimulationSlice {
    fn id(&self) -> SliceId;
    fn version(&self) -> ModelVersion;
    fn required_inputs(&self) -> InputRequirements;
    fn dependencies(&self) -> Vec<SliceDependency>;
    fn validate(&self, scenario: &Scenario) -> Vec<ValidationIssue>;
    fn propose(&self, ctx: &SliceContext<'_>, rng: &mut dyn RngCore)
        -> Result<SliceProposal, SimulationError>;
}

struct SliceProposal {
    events: Vec<ProposedEvent>,
    effects: Vec<ProposedEffect>,
    transactions: Vec<ProposedLedgerTransaction>,
    decisions: Vec<DecisionRequest>,
    scheduled_actions: Vec<ScheduledAction>,
}
```

该接口只是方向；初期可用静态 enum 和具体结构，避免过早动态分发、类型擦除及插件化。需要确定 Slice 能否在一个月内多轮被调用；推荐引擎提供稳定阶段/依赖协议，而非无限循环互调。

`DecisionRequest` 表示需要策略处理的领域选择，不意味着暂停运行。v1 使用场景配置的自动规则解析；交互式等待用户输入属于未来能力。

### 6.3 共享资源与冲突

- Slice 声明 `reads`、`writes` 和必要前置条件；如 Mortgage 写 loan principal 和 cash，Rental 写 occupancy 和 cash。
- **现金变化可加总**，但贷款本金、房屋状况、出租状态等状态 Effects 不应无条件相加。
- 对同一实体的冲突写入，由领域规则合并、串行处理或报错。
- 所有提案带 `slice_id`、`model_version`、`source_event_id`（若有）、`period`。
- 引擎确保事件 ID、流水 ID 稳定且唯一。


## 13. Rust Workspace 与依赖方向

M0-B 已选定最小 workspace 边界，当前实现位于 `crates/`：

| Crate | 职责 | 依赖 |
|---|---|---|
| `property-simulation-domain` (`domain`) | 领域值、实体、不变量与纯领域规则 | 无 workspace crate 依赖 |
| `property-simulation-accounting` (`accounting`) | 科目、凭证、分录、余额和报表投影 | `domain` |
| `property-simulation-models` (`models`) | 领域模型策略与概率分布 | `domain` |
| `property-simulation-simulation` (`simulation`) | 月度生命周期、Slice 协调和运行控制 | `domain`、`accounting`、`models` |

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
- `Money`：十进制定点金额，禁止二进制浮点参与账务计算；`Currency` 首版单币种。
- `RoundingPolicy`：中间计算使用十进制定点精度，最终入账金额按单币种货币最小单位四舍五入（正数 midpoint half-up）；舍入规则是模型版本的一部分。贷款利息按此规则舍入到分。若未来出现多币种或负数舍入场景，再补充相应约定。
- 强类型 ID：场景、运行、Slice、实体、事件、决策、凭证、账户及模型版本 ID。
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
- Slice 提交只读上下文中的候选提案；统一协调器负责验证、冲突处理和原子提交。Slice 不直接改写世界状态或总账。
- 业务事件可以有多个 Effects 和多张有明确语义的凭证；导出/查询层不得把事件、状态变化和会计事实混成一条含义模糊的流水。
