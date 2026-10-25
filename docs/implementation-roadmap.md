# 当前工作：厘清 Core 边界，再接入 Application

当前优先工作是重新梳理 Core 概念、接口和依赖，再迁移实现。旧 crate 布局和 `simulation` 的统一调度 API 不构成目标设计。方向见[架构](architecture.md)和[ADR-33](adr/ADR-33-core-boundaries-and-slice-cohesion.md)；领域行为约束见[领域模型](domain-model.md)。

## Core 重构

- [ ] 定义 `money` 与 `month` 基础值类型 crate 的内容、错误和依赖；按具体业务引用需要再决定是否增加强类型 ID。
- [ ] 明确各 Slice 的数据、配置、不变量、随机输入、计算结果和对账务的接口；把 Slice 专属分布、采样及校验放在其所属行为边界内。
- [ ] 明确 `account`、`journal-entry`、`ledger` 与 `reporting` 的职责和单向类型依赖：Application 在 Month 0 确定账户集合并初始化 Ledger；Ledger 校验并追加平衡分录；reporting 从账簿事实派生报表与核对结果。
- [ ] 定义 Random engine 与随机输入的接口，保持系统熵来源和领域采样行为的责任清楚。
- [ ] 由 Application 定义模拟配置与每月结果的用例接口；Core 不增加 Context、UserSettings 或 Snapshot 聚合类型，也不迁移旧 test-only archive 或 RunControl 作为生产设计。
- [ ] 明确每个 Slice 的实例生命周期与调用输入输出；Application 选择、实例化并独立调用本月所需 Slice，收集结果和 JournalEntry 后更新候选 Ledger。Core 不建立统一跨 Slice 业务聚合器；当前不实现并行调度。
- [ ] 在独立的 `core/` Cargo workspace 建立边界清楚、可独立验证的实现区；逐项迁移规则，不复制未厘清的适配器、调度器或测试支持层。

在接口及依赖图确定前，不继续以旧 `calculate_monthly_slice_result`、`LoanCalculation`、`ActivityConfiguration` 或 `MonthlyResult` 的现有形态规划 Application 接线。

## Application 与 Data Access

Core 边界确定后，再定义并实现 Application 用例。以下产品流程要求仍然有效，具体 Rust 接口以新的 Core 契约为准：

- Application 是 UI、Data Access 与计算能力之间唯一的用例协调点；它不常驻持有模拟进度。
- Application 分别保存初始输入、Slice 配置、停止条件和随机 root，并创建 Month 0 JournalEntry；该分录是初始化账务记录，不伪造成 Month 1 起的模拟期间交易。
- 月度推进先接受并保存用户设置变更，再在下一个月开始前判断全部停止条件（包括 `UserStop`）；满足条件时不计算额外月份。正常推进读取最近成功状态、形成当月有效输入、取得随机输入、执行计算、评估后续停止条件并保存完整结果。
- Application 管理已接受的配置更新和活动启用状态。每个完成月份的实际输入与 Slice 结果按 Month 分别持久化，供续算和审计。
- 不要求统一 Snapshot 聚合。Month 0 与后续月度 JournalEntry 可重建 Ledger 余额；现金流按调用方提供的科目分类映射保留逐分录明细，不要求来源实例 ID。
- 存储或致命计算错误停止本次推进并报告系统错误；不增加重试、恢复、回滚或补偿机制。
- Data Access 使用窄的类型化接口；SQLite 和序列化仅由 Data Access 承担。Presentation 只调用 Application。
- Data Access 接口定义在 storage-neutral `data-access` crate 中，以稳定的语义持久化操作读写 DTO；Application 通过泛型静态分发接收实现，启动组合层创建并注入 SQLite adapter。接口不暴露表级 CRUD；异步 Future 是否要求 `Send` 留待应用运行时确定。
- UI 用例包括设置初始值、推进一个月、获取统计报告和读取指定月份结果。报告从已保存的月度事实与 Ledger 派生，不触发重算。

## 后续验收

- 用可替换的内存适配器验证 Application 的读取、随机输入、计算、停止和保存顺序。
- 用领域测试覆盖 Slice 的输入输出、不变量、来源追踪、随机边界、复式交易及生命周期规则，不依赖 IO。
- 用 SQLite 边界验证序列化、重载和 Ledger/现金流分类明细一致性。
- 检查 Presentation → Application → Data Access/Core 的依赖方向；Core 不依赖 IO、SQLite 或 Presentation。
- 运行 `./scripts/check.sh` 和 `git diff --check`，只报告实际执行的验证。
