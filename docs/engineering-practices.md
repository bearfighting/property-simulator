# 工程规范

## 分层与依赖

- Core（domain、accounting、models、Slices）只实现领域规则，接收显式值并返回结果；不依赖 IO、存储或 presentation。
- Core 的 Slice 独立计算并返回类型化结果及 JournalEntry；Core 不提供统一跨 Slice 业务聚合器。
- Application 负责选择、实例化和协调调用 Slice，汇总当月结果并将分录追加到候选 Ledger；完整月度成功后才接受候选账簿。它也负责 Context、停止条件、跨月流程和数据读写协调。
- 当前不实现 Slice 并行调度；Slice 契约不得要求彼此依赖固定执行顺序。
- Data Access 只由 Application 调用。SQLite 及存储编码局限在适配器内；Application 依赖窄接口，不依赖具体 SQLite 实现。
- CLI/GUI 只解析用户格式、展示结果并调用 Application；不得直接调用 Data Access 或单月计算，也不得复制领域校验。
- 新模块说明其责任、依赖方向和变化原因。优先显式输入/输出函数；仅在确有协作者时引入窄 trait/接口。避免全局可变服务、通用框架和机械拆分。
- 代码标识符不得使用文档中的实施阶段名称或编号。

## 错误和数据边界

- 可由用户修正的结构错误在计算前校验，并尽量一次报告所有可发现问题。
- 计算溢出、内部不变量破坏和基础设施故障均明确失败；不得转成经济结果或部分成功月度结果。
- 单月计算失败不修改调用方 Ledger。应用/存储失败停止本次模拟并报告系统错误；不实现补偿或恢复流程。
- 日志、事件、Effects、Ledger 交易和现金流投影保持职责分离。现金流从 Ledger 推导，按注入的科目分类和分录位置保留明细。

## 测试

- Core 单元测试不依赖 IO，覆盖数学、不变量、边界和错误。
- Core 测试覆盖 Slice 独立结果及 Ledger 不变量；Application 测试覆盖逐 Slice 协调、Ledger 结算、现金流分类明细和失败时无部分月度结果。
- Application 使用 fake/in-memory Data Access 测试调用顺序、context 更新、停止条件、用户停止命令、失败停止和月份连续性。
- SQLite 边界使用集成测试覆盖类型映射、逐月保存、重载状态、Ledger 对账、现金流分类和存储故障报告。
- 测试使用显式输入；固定 seed 只用于确定性测试和诊断。测试时长是 fixture，不是产品约束。
- 不为尚未确定的行为添加占位实现或测试；业务语义改变时同步更新领域文档、ADR 和测试。

## 质量门槛

统一入口为 `./scripts/check.sh`，执行：

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo check --workspace --all-targets`
4. `cargo test --workspace`

使用 Rust stable 和 edition 2024。新增 warning 应修复；局部 lint allow 需说明理由。报告中只列出实际运行的检查及结果。
