# Property Simulation Sandbox (PSS)

**Project Specification v0.4 — 2026-10-12**  
**Status:** 设计基线（尚未实施）  
**Technology:** Rust · GPUI · SQLite · JSON/CSV/XLSX  
**Positioning:** Local-first, monthly, event-driven property lifecycle **simulation data generator**

> 核心原则：**时间定义模拟边界；初始状态定义起点；Slice 定义领域演化；事件与 Effects 解释变化；Ledger 记录经济事实；数据集是产品。**

## 1. 愿景、边界与验收目标

### 1.1 目标

创建一个可配置、可复现的房产生命周期实验沙盒。用户选择一个预设（如首次购房、已持有有贷房产、无贷房产、已出租房产），调整初始状态和未来规则，指定起止月份，运行一个或多个场景。引擎逐月生成房屋状态、业务事件、决策、经济流水与期末快照。必要时暂停供用户决策；最终输出可供其他软件消费的数据集。

**不预测某套真实房屋的必然未来**。默认概率参数是可解释的演示/估计假设，只有经过来源标注和校准的参数才能声称具备相应实证依据。

### 1.2 明确不做

- 投资建议、回报率评分、IRR/NPV、投资组合优化；
- 正式利润表、资产负债表、现金流量表或完整会计准则实现；
- 宏观经济均衡模型、精准天气预测、建筑工程级仿真；
- 云端账户、多人协作、动态插件市场；
- 一开始就建立通用规则 DSL、通用会计 ERP 或分布式事件平台。

**允许**基础的数据完整性摘要（每月收支、余额、未偿本金、组件状态、事件计数）；它们是数据投影，不是财务分析报告。

### 1.3 第一版成功标准

用户可以从任意有效初始状态开始，运行 120/240/360 个月；贷款可在模拟前已存在、模拟中建立、模拟中结清或根本不存在。多个 Slice 叠加生成完整月度数据。发生重要维修时可暂停、选择并继续。相同配置+规则版本+随机种子重跑一致。可以保存/恢复并导出结构化数据。

## 2. 已确定的架构决策（ADR 摘要）

| ID | 决策 | 解释 |
|---|---|---|
| ADR-01 | Simulation 由 `[start_month, end_month]` 控制 | 两端包含；贷款结清、租客搬离等不会结束模拟 |
| ADR-02 | 统一 **monthly tick** | 一月一个事务性结算周期；20 年 = 240 tick |
| ADR-03 | 从**任意有效初始快照**开始 | 不要求补齐模拟前的购房/还贷事件 |
| ADR-04 | Scenario Preset + 用户覆盖 | 初始事实与未来假设分离；每个 Slice 校验自己的输入 |
| ADR-05 | Slice 独立计算、统一提交 | Slice 不直接写共享 State/Ledger；输出 Proposals/Effects |
| ADR-06 | Event、Effect、Ledger 分离 | 事件说明发生了什么；Effect 改变状态；Ledger 记录经济事实 |
| ADR-07 | 现金流按月入账 | 保留每笔明细及其来源，月末产生聚合视图 |
| ADR-08 | 核心产物是 Dataset | JSON 完整交换，CSV/XLSX 扁平导出；不生成投资报告 |
| ADR-09 | 随机性可复现 | 稳定排序、固定算法版本、按 Slice/实体/用途隔离随机流 |
| ADR-10 | 三阶段开发 | I 领域 Slice；II 集成执行/决策/存储；III GPUI |
| ADR-11 | 初期 Rust 静态注册 | 暂不做动态插件、通用配置语言或异步服务架构 |
| ADR-12 | 物理/经济过程允许相关 | 模块解耦≠随机变量独立；通过共享状态、环境及显式因果关联 |

## 3. 核心概念与术语

| 概念 | 含义 | 示例 |
|---|---|---|
| Scenario | 完整模拟输入：时间、初始事实、启用 Slice、规则、种子 | 已持有房产，2030–2050 |
| Preset | 可覆盖的起点模板 | 已出租、无贷款 |
| Initial World State | 月初已存在的事实快照 | 贷款本金 275,000，屋顶 12 年 |
| Assumptions | 未来演化的模型与参数 | 利率重置策略、维修概率 |
| Slice | 单一领域的规则和内部状态 | Mortgage、Maintenance |
| Process | 连续/周期性的状态演化 | 屋顶老化、贷款计息 |
| Event | 已发生的离散事实 | 维修需求、贷款结清 |
| Decision | 外部选择及其记录 | 修补/更换/延期 |
| Effect | 经过验证、将要应用的状态或经济变化 | 本金减少、屋顶状况改善 |
| Ledger Transaction | 同一业务动作的经济记录集合 | 月供的现金、本金、利息分解 |
| Monthly Period | 月度执行与结算容器 | 2032-06 |
| Snapshot | 期初或期末世界状态 | 2032-06 closing state |
| Dataset | 可交换的模拟结果 | 事件、流水、状态、配置、元数据 |

**生命周期区分：** Simulation Lifecycle 是固定时间区间；Property Lifecycle、Mortgage Lifecycle、Lease Lifecycle、Component Lifecycle 是彼此独立的领域过程。出售房产可以停止其运营 Slice，但默认仍推进模拟至 `end_month`（之后可能只有现金状态或空活动）。

## 4. Scenario Builder：像 CRPG 捏脸一样创建世界

### 4.1 用户流程

1. **选择预设**：首次购房、已持有且有贷款、无贷自住、已出租。
2. **设置时间范围**：起始年月、结束年月；默认 20 年，但完全可修改。
3. **编辑初始事实**：房产、现金、贷款、组件、租约等；只显示已启用 Slice 所需字段。
4. **编辑未来假设**：利率、通胀、天气、维修模型、决策策略、随机种子。
5. **预览和校验**：错误阻止运行；警告说明参数可能不现实或缺乏校准。
6. **冻结 Scenario**：保存有效输入、版本和配置哈希；开始运行。

### 4.2 两类独立预设

- **Scenario Preset**：起始世界状态；例如 `existing-mortgaged-home`。
- **Model Preset**：未来演化规则；例如 `baseline-maintenance`、`high-cost-environment`。

二者可以交叉组合。默认值分层：系统默认 → 地区配置（可选）→ 场景预设 → 用户覆盖。最终展开为**完整、不可变的运行配置**，不能在历史运行中静默修改。

### 4.3 初始事实与假设分开

**事实：** 模拟起始时的现金、资产估值、贷款未偿本金、贷款条款、组件年龄/状况、租赁状态。  
**假设：** 未来故障风险、成本分布、通胀、租客搬离概率、未来利率变化、维修策略。

初始房产估值是起点的估值数据，不是本月购房收入。期初现金不是本月资金流入；期初贷款也不是本月新增借款。

### 4.4 预设示例（示意参数，非市场事实）

```yaml
schema_version: 1
id: existing-mortgaged-home
scenario:
  start_month: 2030-01
  end_month: 2049-12
  master_seed: 42
initial_state:
  property:
    ownership_status: owned
    estimated_value: { amount: '600000.00', currency: CAD }
  cash_accounts:
    - { id: operating, opening_balance: '25000.00', currency: CAD }
  loans:
    - id: mortgage-1
      outstanding_principal: '275000.00'
      annual_rate: '0.045'
      payment_amount: '1739.00'
      remaining_amortization_months: 240
      next_payment_month: 2030-01
      rate_type: fixed
      term_end_month: 2034-12
  components:
    - { id: roof-1, kind: roof, age_months: 144, condition: 0.75 }
enabled_slices:
  - mortgage
  - property-tax
  - maintenance
assumptions:
  maintenance_policy: manual_above_threshold
```

上述贷款数值仅演示结构，**并未保证还款额与其他字段精确匹配**；真实运行必须执行条款一致性校验，并明确计息/还款约定。

### 4.5 校验规则

- `start_month <= end_month`；支持合理的最大期限限制。
- 启用 Mortgage 且本金 > 0 时，必须具备利率、付款计划、计息约定、下一次还款时间、必要的续约假设。
- 已有租客时必须提供租金和支付周期，或明确选择默认规则。
- 组件状况必须在合法区间；安装年龄与状态可以相关，但不强制仅由年龄决定。
- 购房交易不得与已经入账的期初房产资产、贷款和现金重复计入。
- 初始余额不得被记录成当月 Ledger Transaction。
- 货币一致性；跨币种暂不支持或显式禁用。
- 启用 Slice 声明其所需实体和配置字段；缺失即报错。

## 5. 时间模型与月度执行语义

### 5.1 月份

`YearMonth` 采用 YYYY-MM；`[start_month, end_month]` 为包含边界。月份而非天是模型的基本精度。无日级真实付款日、罚息或天气逐日轨迹的保证。

### 5.2 月度状态

每个 `MonthlyPeriod` 具有：month、opening snapshot reference、proposed/committed events、decisions、state effects、ledger transactions、closing snapshot reference、monthly totals、status。

### 5.3 规范化执行阶段

1. **Open**：读取上月 closing state（首月读取 initial state）。
2. **Context**：建立本月共享环境输入和只读视图。
3. **Accrue/Evolve**：计算月度过程（例如老化、计息），生成候选 Effects。
4. **Propose**：各 Slice 根据状态、计划和随机输入提出事件、经济流水与决策需求。
5. **Resolve**：按依赖和优先级处理跨 Slice 关系、验证前置条件、解决冲突。
6. **Decide**：若需要交互，冻结待结算 Period 并暂停；提交决策后重新验证受影响候选项。
7. **Commit**：原子应用状态 Effects 和 Ledger Transactions；确保只提交一次。
8. **Close**：计算月度汇总、生成月末快照、推进到下月。

**注意：** 这是语义阶段，不要求所有 Slice 都在同一时刻读取旧状态后无条件相加。存在依赖时必须按依赖拓扑顺序/显式阶段读取已提交的中间结果，或重新计算受影响 Proposal。

### 5.4 同月因果顺序

示例：本月租客搬离 → 本月租金是否收取，由租赁规则决定；不得简单让两个 Slice 各自输出矛盾结果。推荐每个 Slice 声明读/写资源、前置条件和依赖。执行器先确定相关事实，再产生依赖它的后续经济 Effects。

MVP 约束：同月只允许有限层数的因果展开；需要更细粒度的多次交互可排到下一月。必须记录这种时间分辨率带来的近似。

### 5.5 暂停与结算

若本月存在待决策项，**当月尚未 Close**；继续后只能提交一次。默认可批量呈现本月决策，但若一项选择影响其他决策的有效性，必须重新验证/重新生成相关候选事件。用户可选择默认自动策略，避免 240 个月被频繁打断。

### 5.6 正常终止

只有处理完 `end_month` 后状态变为 `Completed`。提前停止由显式取消、失败或用户设置的可选场景终止规则产生，不能由贷款结清隐式触发。

## 6. Slice 架构与契约

### 6.1 目标

每个 Slice 负责自己的领域状态、规则、事件类型、配置校验与测试；引擎负责时间、排序、依赖协调、随机流、提交、保存和输出。**Slice 不得直接修改共享世界或 Ledger。**

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

### 6.3 共享资源与冲突

- Slice 声明 `reads`、`writes` 和必要前置条件；如 Mortgage 写 loan principal 和 cash，Rental 写 occupancy 和 cash。
- **现金变化可加总**，但贷款本金、房屋状况、出租状态等状态 Effects 不应无条件相加。
- 对同一实体的冲突写入，由领域规则合并、串行处理或报错。
- 所有提案带 `slice_id`、`model_version`、`source_event_id`（若有）、`period`。
- 引擎确保事件 ID、流水 ID 稳定且唯一。

## 7. 业务 Slice 路线图与验收

### S1. Property Acquisition & Mortgage（第一个纵向 Slice）

**支持两种入口：**

- **New Acquisition**：购房发生于模拟内；生成购买、首付、交易费用、贷款发放及房产所有权状态变化。
- **Existing Ownership**：模拟开始时已持有；直接载入房产资产和未偿贷款的 Opening State，**不伪造购房历史**。

**状态：** 房屋持有状态、贷款余额、利率、付款金额、计息规则、剩余摊还期、利率条款到期月、付款计划、贷款状态。

**月度行为：** 计算利息、拆分本金/利息、扣除现金、更新本金；最后一期调整防止超额还本；结清时产生 `mortgage.paid_off` 事件；之后不再扣月供，但 Simulation 继续。

**边界：** 固定利率、固定月供、单笔贷款先实现；加拿大式半年复利折算、续约/浮动利率、提前还款、再融资等作为显式子里程碑，不能未经定义就混用不同金融计息约定。

**关键测试：**
- 初始余额正确；购房交易不重复入账；
- 每月 `payment = principal + interest`（按统一舍入约定）；
- 本金不为负；最终一期正确结清；
- 模拟在结清后仍运行；
- 从贷款第 80 个月状态开始与同等条款的连续运行结果一致（在完整必要状态相同前提下）；
- 零贷款情形不产生还款。

### S2. Property Tax & Insurance

按年度、季度或月度规则产生应付/支付；首版以月度归集。税费与保险分开分类；可配置金额、周期、涨幅与起始月份。**不将年度费用默认均摊为每月真实支付**，除非选择预算化/月缴规则。

测试：不同支付周期、年界、起始月份、金额调整、与 S1 同月现金合计。

### S3. Regular Maintenance & Exterior

除草、除雪、花园维护、定期检查。规则可为季节性、周期性或状态阈值；雪量等可由简化环境变量生成。记录事件与费用；无需日级天气仿真。

测试：季节、阈值、无雪不触发阈值除雪、自动策略、费用分类。

### S4. Component Deterioration & Repair

屋顶优先；再扩展热水器、供暖、管道。状态含年龄、状况、损伤、维修历史；自然演化与随机故障分开。事件可触发修补、更换、延期决策；维修完成与付款可跨月。维修不等于等额增加资产市场价值。

测试：故障概率、维修后状态、延期后风险、事件因果、现金和状态一致性。

### S5. Occupancy & Rental

自住/出租/空置状态，租金计划、租客搬离、重新出租、空置成本。租客离开影响后续收租规则；是否当月全额收租必须有显式政策。

测试：空置无正常租金、租约起止、状态转换、跨 Slice 影响。

### S6. Utilities & Operating Costs

水电暖、服务费用；月度固定/季节/使用量相关模型。与气温、入住状态相关，但保持模块隔离。

测试：季节性、空置用量、异常费用和模型版本。

### S7. Valuation & Disposal（可选扩展）

估值变化属于资产状态而非现金收入。出售发生时，生成成交现金流、交易费用、清偿贷款、持有状态变化；出售后停止房产运营 Slice。**默认仍推进至模拟结束月份**。

测试：出售清算不重复记账；已出售房产不再产生物业运营事件；模拟不因出售自动终止。

### 7.1 Slice 实施原则

每完成一个 Slice，先通过独立测试，再与已有 Slice 运行同一个 24/240 月场景。避免所有 Slice 写完后才第一次整合。建议第一个 CLI 纵向切片包括：Scenario → S1 → Monthly Runner → Ledger → JSON output。

## 8. 事件、过程、决策与 Effects

### 8.1 事件边界

- **Process**：持续老化、计息、环境演化；不需要每月生成“老化发生”事件。
- **Event**：离散事实；如 `roof.leak_detected`、`mortgage.paid_off`。
- **Action/Decision**：用户或策略选择；如 `repair`、`replace`、`postpone`。
- **Effect**：状态修改、经济义务、现金交易、未来计划变化。

### 8.2 事件字段

```rust
struct SimulationEvent {
    id: EventId,
    period: YearMonth,
    kind: EventKind,
    source_slice: SliceId,
    subject: EntityId,
    cause_ids: Vec<EventId>,
    payload: EventPayload,
    trace: Option<EventTrace>,
}
```

建议稳定的 namespaced type ID：`mortgage.payment_completed`、`mortgage.paid_off`、`roof.leak_detected`、`maintenance.snow_removed`、`rental.tenant_departed`。事件类型与生成器一对多：同一个屋顶损坏事件可以由老化或暴风雪生成。

### 8.3 触发类型

`Scheduled`、`Periodic`、`Conditional`、`Stochastic`、`Decision`。触发机制不是互斥业务类别。

### 8.4 事件过滤原则

模拟直接影响现金、未来现金、物理/经济状态或其他重要事件条件的变化。普通晴天、植物开花等若不改变模型状态或风险，可作为环境输入而非正式事件。

### 8.5 决策策略

`Automatic`、`Interactive`、`ConditionalThreshold`。重大维修可默认暂停；小额例行费用自动处理。决策记录包含请求、可选项、所选项、依据、关联事件、产生的后续安排。手动改变已结算历史须创建新分支，不直接改写旧记录。

### 8.6 Event Trace

可选保存模型版本、输入状态摘要、风险/阈值、随机抽样值、计算出的严重程度和成本参数。目标是可解释和可复现，而非展示伪精确的现实预测。

## 9. 随机性与概率模型

### 9.1 发生与后果分离

故障发生概率、损坏严重度、维修费用、维修持续月份应独立建模。风险不是事件；事件不等于成本。

### 9.2 首批工具

- Bernoulli：月度发生/不发生；
- Poisson：事件到达强度（适用时）；
- Weibull：年龄相关的失效时间/危险率；
- Categorical：严重程度；
- Lognormal 或有界分布：非负右偏费用；
- Scheduled/Threshold：确定性与条件事件。

Weibull 失效分布：`F(t)=1-exp(-(t/eta)^beta)`，其中 t/eta 使用一致时间单位。月度条件概率为：

`p = 1 - exp(-[((t+Δ)/eta)^beta - (t/eta)^beta])`，Δ = 1/12 年（若 t 以年计）。

**不保证**有限 20 年内一定故障；更不把 Weibull 当作物理状况的唯一老化公式。

### 9.3 共享环境与相关性

天气、通胀、入住率等由共享 Environment State 提供；多个 Slice 读取它们，因此事件可相关，但 Slice 不互相调用。环境数据可按月生成，不必每次变化都写入 Event Log。

### 9.4 可复现性

固定主种子、PRNG 算法版本、每个 Slice/实体/随机用途的独立流、稳定排序和舍入规则。保存模型配置快照、engine version、ruleset version、schema version、seed。**历史回放**应用保存的 Effects；**重跑**重新执行模型与随机抽样，两者分开。

## 10. Ledger：必要但克制

### 10.1 为什么要 Ledger

仅有月度 `income` / `expense` 无法正确表示购房、贷款发放、归还本金、利息、维修费用与估值变化。Ledger 是经济事实层，不等于 Financial Report。

### 10.2 建议最小账户

- `CashAccount`：现金余额；
- `LoanAccount`：未偿贷款本金；
- `PropertyAsset`：房产账面/参考估值状态（明确字段语义）；
- `ExpenseCategory`、`IncomeCategory`：分类与追溯。

可采用 **typed economic effects + ledger transactions** 的轻量模型，而不是第一版就强制完整借贷复式记账。必须有统一的符号/余额变更约定和每笔业务的平衡检查；如果后续升级复式记账，需明确借贷方向与迁移方式。

### 10.3 交易与分录

```rust
struct LedgerTransaction {
    id: TransactionId,
    period: YearMonth,
    source_slice: SliceId,
    source_event_id: Option<EventId>,
    entries: Vec<LedgerEntry>,
}

struct LedgerEntry {
    account_id: AccountId,
    change: Decimal,
    classification: EntryClassification,
}
```

`change` 的符号表示该账户**业务余额**的增减，不是会计借贷方向。若采用轻量余额变化账本，不应要求各账户变化简单相加为零：不同类型账户的经济含义不同；应对每种交易定义领域一致性校验。

例如月供 1,800，其中本金 1,250、利息 550：现金 -1,800；贷款本金 -1,250；利息费用 550（费用分类/事实）。`payment = principal + interest`，而不是把 1,800 全算作经营费用。

### 10.4 月度结算

每月保留：

- Opening balances；
- 完整交易明细；
- Operating cash inflows/outflows；
- Financing cash flows（贷款、注资等，分开分类）；
- Net cash change；
- Closing balances；
- Loan outstanding；
- World State snapshot。

公式：`closing_cash = opening_cash + sum(cash_changes)`。现金流与资产估值变化分开。年度费用按**实际配置的支付月**入账；不能为了方便自动伪造均匀的月度支付。

### 10.5 资金不足

首版可支持 `Unlimited/AutoFund` 和 `Limited`。外部注资属于融资活动，不算经营收入；Limited 模式可触发决策（推迟维修、追加资金）。不要将资金不足直接当作引擎崩溃。

### 10.6 无 Financial Reports

不实现利润表、资产负债表、正式现金流量表、IRR、NPV 或收益率 Dashboard。允许原始月度投影和数据质量检查。外部分析程序负责解释结果。

## 11. 世界状态与存储

### 11.1 World State

```rust
struct WorldState {
    property: PropertyState,
    components: Vec<ComponentState>,
    occupancy: OccupancyState,
    loans: Vec<LoanState>,
    accounts: Vec<AccountState>,
    environment: EnvironmentState,
    slice_states: SliceStates,
}
```

初始状态可以从模拟中途接入，要求**充分状态**：仅知道贷款本金但不知道条款、或仅知道屋顶年龄但不知道需要的状况参数，可能不足以继续某个 Slice。

### 11.2 历史与快照

- Event Log：不可变的事件历史；
- Decision Log：选择与策略历史；
- Ledger：经济明细；
- State Effects：状态变化；
- Monthly Snapshots：至少月末状态；
- Runtime Checkpoint：用于恢复未完成月份的调度队列、待决策状态和必要 RNG 状态。

运行存档与数据导出不是同一件事：JSON 数据集可用于外部消费，但恢复精确执行还需要 runtime checkpoint。

### 11.3 持久化

SQLite 用于项目、配置、事件、流水、快照、运行状态；MVP 可先内存执行并输出 JSON，再引入 SQLite。提交必须保证事件、Effects、Ledger、快照的原子性和幂等性。

## 12. 输出数据契约

### 12.1 JSON（主要格式）

```json
{
  "schema_version": 1,
  "simulation": {
    "start_month": "2030-01",
    "end_month": "2049-12",
    "seed": 42,
    "engine_version": "0.1.0",
    "ruleset_version": "1"
  },
  "scenario": { "preset_id": "existing-mortgaged-home" },
  "initial_state": {},
  "events": [],
  "decisions": [],
  "effects": [],
  "ledger_transactions": [],
  "monthly_snapshots": [],
  "final_state": {}
}
```

实际导出必须包含展开后的完整配置或可验证的配置快照，而不只是 `preset_id`。金额用十进制字符串，时间使用 `YYYY-MM`，ID 稳定，记录版本与单位。

### 12.2 CSV

`events.csv`、`effects.csv`、`decisions.csv`、`ledger_transactions.csv`、`ledger_entries.csv`、`monthly_snapshots.csv`、`scenario_metadata.csv`；必要时打包 ZIP。

### 12.3 XLSX

与 CSV 对应的工作表，供 Excel/LibreOffice 等读取。XLSX 是结构化表格导出，不是预制财务报表。

### 12.4 导出契约

稳定字段、显式单位、明确空值、schema version、配置哈希、事件因果 ID、Slice ID、模型版本；导出前检查月份连续、账户余额对账、贷款本金一致、重复 ID 与非法引用。

## 13. Rust Workspace 与依赖方向

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

依赖方向：`simulation-core` 不依赖 GPUI 或 real-estate；领域 Slice 使用核心契约；Ledger 可被核心或应用层协调，但不能引入循环依赖；Desktop 只调用应用接口。实际 crate 拆分按编译边界与复杂度逐步演化，避免第一天创建过多空 crate。

**候选库**：`serde`、`serde_json`、`rand`/`rand_distr`、`rust_decimal`、`time`、`thiserror`、`tracing`、`clap`、`rusqlite` 或 `sqlx`、`csv`、`rust_xlsxwriter`、GPUI。具体版本与 Linux 平台兼容性在初始化时验证。

## 14. GPUI Desktop 产品设计

### 14.1 核心页面

1. **Project/Scenario Library**：创建、复制、打开实验。
2. **Scenario Builder**：预设选择、事实与假设分区、启用 Slice、校验、开始。
3. **Monthly Timeline**：按年折叠/按月展开，事件、结算、状态变化。
4. **Event Inspector**：来源、条件、风险、因果链、Effects、相关 Ledger。
5. **Decision Panel**：展示维修/融资等选项，暂停与继续。
6. **Ledger Explorer**：月度明细、账户余额、交易来源；不展示投资建议。
7. **Export/Save**：数据集格式、范围、保存/恢复。

### 14.2 交互原则

- 月度为最小导航单位，默认按年汇总；
- 常规付款可折叠，重大事件突出；
- 每月显示 opening/closing balance、事件数和现金变化；
- 高级概率参数放在可展开的技术面板；
- 默认自动处理小额事件，避免 20 年模拟频繁暂停；
- GUI 不直接修改运行中的 Core 状态；配置修改创建新运行/分支；
- 长运行放后台线程，向 UI 发送进度/快照，避免阻塞主线程。

### 14.3 Scenario Builder 表单生成

预设是版本化数据；每个 Slice 暴露默认配置、字段元数据、必填条件和校验逻辑。UI 可根据元数据构建大部分表单，但跨字段逻辑由 Rust 校验器负责。首版允许部分表单手写，不要求完全自动生成动态表单。

## 15. 三个开发阶段（正式计划）

### Phase I — Domain Models, Vertical Slices & CLI

**目标：** 验证月度模型、财务正确性与 Slice 可组合性；不依赖 GPUI。

**I.0 基础最小内核**：YearMonth、Scenario、WorldState、Effect、Ledger、Monthly Runner、JSON CLI 输出；只实现 S1 所需接口。

**I.1 S1 购房/已有房产与贷款**：完整月供、结清、中途开始、无贷测试。

**I.2 S2 税费保险**：与 S1 叠加验证月度现金余额。

**I.3 S3 常规维护**：季节/周期/条件事件，验证事件注册与状态输入。

**I.4 S4 屋顶老化与维修**：加入第一种随机风险模型和最小决策流程。

**I.5 S5 租赁**：跨 Slice 的现金流和状态影响；S6/S7 可按优先级追加。

每个 Slice 都要求：独立单元测试、24 月 golden fixture、240 月组合回归、参数校验、事件/经济记录可追溯。

**Phase I 完成标准：** CLI 可从新购/已持有预设运行完整月度场景；S1–S5 可组合；现金与贷款状态对账；随机种子可复现；输出可验证 JSON。

### Phase II — Integration, Reliability, Persistence & Data Contracts

**目标：** 把可用的模型整合为可靠实验引擎。

- 正式 Slice dependency/resolution protocol；
- 统一随机流与模型版本；
- 交互式暂停、恢复、自动策略；
- Event Trace 与历史 Replay；
- SQLite 存档、checkpoint、恢复；
- JSON schema、CSV/XLSX 导出；
- 统计测试、跨 Slice 不变量、可选 branch；
- 运行性能和内存验证。

**完成标准：** 中断/恢复不会重复扣款；重放与原始记录一致；可导出完整数据集；随机模型统计结果与其定义相符。

### Phase III — GPUI Desktop Application

**目标：** 完整本地桌面实验工作流。

- Scenario Library + CRPG 风格 Builder；
- 参数覆盖、校验与预览；
- Timeline、Inspector、Ledger Explorer；
- 运行、暂停、继续、决策；
- 存档、导出、错误提示；
- Linux 桌面优先，其他平台视 GPUI 支持情况验证。

**完成标准：** 用户无需 CLI，即可创建已有房产或新购场景，调整参数，完成 20 年模拟，处理决策并导出数据。

## 16. 测试策略与正确性要求

### 16.1 单元/领域测试

- 月份边界、闰年不影响月 Tick、期末包含规则；
- 贷款利息、舍入、最终一期、零余额、条款一致性；
- 物业税/保险周期、现金入账月份；
- 空置时租金、维修时状态变化、出售后停止运营；
- 不允许无效实体 ID、重复交易、非法组件状态。

### 16.2 集成与 Golden Tests

- S1 独立 240 月；
- S1+S2+S3 同月叠加；
- S4 维修暂停/恢复后现金仅扣一次；
- 已有贷款从第 N 月开始 vs 等价初始快照；
- 贷款结清后仍继续税费与维护；
- 购房交易不重复形成期初资产；
- 同月出租状态变化与租金规则一致。

### 16.3 统计测试

固定大量 seed 验证 Bernoulli/Weibull 等抽样符合理论范围；检查维修严重程度和费用的合法性；不同随机流互不因无关 Slice 的加入而变化。统计测试不等于现实数据校准。

### 16.4 不变量与对账

`closing_cash = opening_cash + cash_delta`；本金减少不超过余额；付款拆分正确；每月只结算一次；同一事件不重复执行；事件原因引用存在；所有 Slice 状态满足约束；`start_month...end_month` 月份连续。

### 16.5 模型可信度

明确区分 **软件正确性**、**统计正确性**、**现实校准有效性**。默认参数注明 `demonstration` / `estimated` / `calibrated` / `user_defined`，保存来源和版本。

## 17. 风险、开放问题与建议默认值

| 问题 | v0.4 默认决定 | 未来可扩展 |
|---|---|---|
| 月内事件顺序 | 显式阶段+依赖、同月有限因果深度 | 更细粒度时间 |
| 贷款计息约定 | 第一版选定一种并写测试 | 加拿大/美国多种计息规则 |
| 账本模式 | 轻量 typed ledger + 领域对账 | 完整复式记账 |
| 多币种 | 单币种 | 汇率与多币种 |
| 资产估值 | 初始参考值，可选简单变化模型 | 真实市场校准 |
| 随机事件 | 少量可解释模型 | 丰富分布与相关性 |
| 交互式决策 | 重大事件暂停，自动策略处理小额 | 策略编辑器 |
| 事件扩展 | Rust 静态注册 | 外部插件/DSL |
| 运行性能 | 单机、月 Tick、同步核心 | 批量并行模拟 |
| Branch | Phase II 可选 | 多场景对照实验 |
| 现金不足 | 自动注资/有限资金策略 | 完整融资模型 |

**必须在 S1 编码前敲定的细节：** 贷款利率计息与复利约定、月供舍入规则、购买发生在起始月的哪个结算阶段、Opening Balance 的符号与账户语义。这些决定直接影响所有后续测试的期望值。

## 18. 推荐的第一份可执行样例

**Scenario A — New Purchase**
- 2030-01 至 2049-12；2030-01 购入房产；首付+贷款；固定月供；无租客；从第二个 Slice 开始加入税费。

**Scenario B — Existing Mortgaged Home**
- 2035-01 至 2054-12；已经持有房产；存在未偿贷款；不产生历史购房交易；贷款可能在模拟中途结清，模拟继续。

**Scenario C — Mortgage-free Rental**
- 2030-01 至 2049-12；无贷款，已有租客；租金、税费、维护叠加；后续可加入租客搬离与屋顶维修。

所有样例都应保留固定 seed 和可版本化 golden 输出，作为持续集成的基准。

## 19. 建议下一步任务清单

1. 在仓库创建本规范及 ADR 目录。
2. 写 `YearMonth`、`SimulationConfig`、`InitialWorldState`、`MonthlyPeriod` 的最小类型。
3. 选择并固定 S1 的贷款计息、月供和舍入规则。
4. 定义 `LedgerTransaction`、账户余额变更语义与对账函数。
5. 实现 Scenario A/B 两个 CLI 样例，并用固定 JSON golden tests 验证。
6. 实现 `SliceProposal` 与统一月度提交协议。
7. 再添加 S2 税费保险，验证 Slice 叠加没有重复交易。

---

## 附录 A：项目一句话定义

**Property Simulation Sandbox 是一个以月度时间段为基础、从任意有效房产初始状态出发、由可组合业务 Slice 驱动、生成可复现事件历史、经济 Ledger 和世界状态数据集的本地模拟工具。**

## 附录 B：最重要的边界

- Simulation 结束由**时间范围**决定，不由贷款结清决定。
- Initial State 是**起始事实**，不等于过去事件历史。
- Slice 是**领域逻辑单元**，不是各自独立提交的账本。
- Event 是**事实**，Effect 是**变化**，Ledger 是**经济记录**。
- Monthly Snapshot 是**数据投影**，不是 Financial Report。
- GPUI 是**实验操作界面**，不是投资分析 Dashboard。
