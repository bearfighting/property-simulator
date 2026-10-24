# 当前工作：厘清 Core 边界，再接入 Application

当前优先工作是重新梳理 Core 概念、接口和依赖，再迁移实现。旧 crate 布局和 `simulation` 的统一调度 API 不构成目标设计。方向见[架构](architecture.md)和[ADR-33](adr/ADR-33-core-boundaries-and-slice-cohesion.md)；领域行为约束见[领域模型](domain-model.md)。

## Core 重构

- [ ] 定义 `money` 与 `month` 基础值类型 crate 的内容、错误和依赖；按具体业务引用需要再决定是否增加强类型 ID。
- [ ] 明确各 Slice 的数据、配置、不变量、随机输入、计算结果和对账务的接口；把 Slice 专属分布、采样及校验放在其所属行为边界内。
- [ ] 明确 `account`、`transaction`、`posting`、`ledger` 与 `reporting` 的职责和单向类型依赖：Application 在期初确定账户集合并初始化 Ledger；posting 校验候选交易；Ledger 只追加已接受交易；reporting 从账簿事实派生报表与核对结果。
- [ ] 定义 Random engine 与随机输入的接口，保持系统熵来源和领域采样行为的责任清楚。
- [ ] 定义 Context 与 Snapshot 的类型化语义、更新责任、生命周期和持久化映射；不迁移旧 test-only archive 或 RunControl 作为生产设计。
- [ ] 明确每个 Slice 的实例生命周期与调用输入输出；Application 选择、实例化并独立调用本月所需 Slice，收集结果和 JournalEntry 后更新候选 Ledger。Core 不建立统一跨 Slice 业务聚合器；当前不实现并行调度。
- [ ] 在独立的 `core/` Cargo workspace 建立边界清楚、可独立验证的实现区；逐项迁移规则，不复制未厘清的适配器、调度器或测试支持层。

在接口及依赖图确定前，不继续以旧 `calculate_monthly_slice_result`、`LoanCalculation`、`ActivityConfiguration` 或 `MonthlyResult` 的现有形态规划 Application 接线。

## Application 与 Data Access

Core 边界确定后，再定义并实现 Application 用例。以下产品流程要求仍然有效，具体 Rust 接口以新的 Core 契约为准：

- Application 是 UI、Data Access 与计算能力之间唯一的用例协调点；它不常驻持有模拟进度。
- 初始设置保存 context、期初状态、开账 Ledger 和应用版本；期初事实不能伪造成模拟期间的交易。
- 月度推进先接受并保存用户设置变更，再判断停止命令；停止命令不计算额外月份。正常推进读取最近成功状态、形成当月有效输入、取得随机输入、执行计算、评估停止条件并保存完整结果。
- Context 保留初始设置、最新设置、用户输入和活动启用状态。每个完成月份保留该月实际输入与结果，供续算和审计。
- Snapshot 保存完整月度 Slice 结果及其类型化状态变化。Ledger 开账事实与交易可重建余额；现金流按调用方提供的科目分类映射保留逐分录明细，不要求来源实例 ID。
- 存储或致命计算错误停止本次推进并报告系统错误；不增加重试、恢复、回滚或补偿机制。
- Data Access 使用窄的类型化接口；SQLite 和序列化仅由 Data Access 承担。Presentation 只调用 Application。
- UI 用例包括设置初始值、推进一个月、获取统计报告和读取指定月份完整结果。报告从已保存的 Snapshot 与 Ledger 派生，不触发重算。

## 后续验收

- 用可替换的内存适配器验证 Application 的读取、随机输入、计算、停止和保存顺序。
- 用领域测试覆盖 Slice 的输入输出、不变量、来源追踪、随机边界、复式交易及生命周期规则，不依赖 IO。
- 用 SQLite 边界验证序列化、重载和 Ledger/现金流分类明细一致性。
- 检查 Presentation → Application → Data Access/Core 的依赖方向；Core 不依赖 IO、SQLite 或 Presentation。
- 运行 `./scripts/check.sh` 和 `git diff --check`，只报告实际执行的验证。
