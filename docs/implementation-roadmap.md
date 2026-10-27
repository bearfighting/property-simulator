# 当前工作：接入 Application 与 Data Access

Core 基础实现已达到当前阶段目标，作为后续 Application 接线的工作基线。遇到具体需求或实现问题时，再按需调整 Core 接口和职责；不再把 Core 全面重构列为当前前置工作。相关方向见[架构](architecture.md)和[ADR-33](adr/ADR-33-core-boundaries-and-slice-cohesion.md)，领域规则见[领域模型](domain-model.md)。

## Core 基线（当前阶段完成）

- Money/Month 基础类型、Slice 契约与计算、Accounting/Ledger/Reporting、随机输入和停止条件均已在当前 Core workspace 建立实现。
- Core 不持有模拟级 Context/Snapshot，也不提供跨 Slice 调度器；Application 负责组合各能力。
- 这是当前阶段的完成判断，不表示 Core 永久冻结。后续按具体用例、领域规则或验证结果增量调整。

## Application 与 Data Access

当前工作转向定义并实现 Application 用例，并按调用方的实际需要接入已完成的 Data Access 契约。以下产品流程要求仍然有效，具体 Rust 接口以当前 Core 契约为准：

- Application 是 UI、Data Access 与计算能力之间唯一的用例协调点；它不常驻持有模拟进度。
- Application 分别保存初始输入、Slice 配置、停止条件和随机 root，并创建 Month 0 JournalEntry；该分录是初始化账务记录，不伪造成 Month 1 起的模拟期间交易。
- 模拟初始化后配置在本次模拟中固定，不提供模拟期间修改配置的功能；月度推进只读取初始化时保存的配置。用户可停止模拟，或推进一个完整月份。创建另一项模拟属于独立流程，不改变现有模拟配置。
- 月度推进在下一个月开始前判断初始化时设定的自动停止条件；满足条件时不计算额外月份。用户结束运行由内存生命周期协调器处理。正常推进读取最近成功状态、形成当月输入、取得随机输入、执行计算、评估后续停止条件并保存完整结果。
- 每个完成月份的实际输入与 Slice 结果按 Month 分别持久化，供续算和审计。配置在运行期间不可更新，因此不需要配置版本、生效月份或配置变更历史。详见 [ADR-70](adr/ADR-70-immutable-simulation-configuration.md)。
- 不要求统一 Snapshot 聚合。Month 0 与后续月度 JournalEntry 可重建 Ledger 余额；现金流按调用方提供的科目分类映射保留逐分录明细，不要求来源实例 ID。
- 存储或致命计算错误停止本次推进并报告系统错误；不增加重试、恢复、回滚或补偿机制。
- Data Access 使用窄的类型化接口；SQLite 和序列化仅由 Data Access 承担。desktop 组合入口负责连接 Presentation UI、Application 与 Data Access。
- 当前先完成初始化模拟的 GPUI 垂直切片：`presentation/gpui/ui` 依赖 GPUI Kit 和只读生命周期状态类型，并提交普通表单数据；`presentation/gpui/desktop` 是程序组合入口，负责创建 SQLite pool、安装 schema、构造账户目录，并协调 UI 与 Application。UI 不直接依赖 Application 或 Data Access，也不推进生命周期状态。入口通过 Tokio runtime 驱动 SQLx，提交结果异步返回 UI。
- Desktop 的生命周期协调器持有当前 simulation ID 和内存状态；系统操作可结束本次运行或确认后重置到新的初始化流程。重置不删除旧的 SQLite 数据，当前 UI 暂不提供旧模拟的选择或浏览。
- Data Access 的 DTO、SQLite schema、storage-neutral store traits 和 SQLite adapter 已实现。接口按模拟基础事实、设置、月度事实分组，并提供组合 `DataAccess` trait；较小 work unit 可只依赖所需分组。接口覆盖 schema 中模拟、随机 root、账户、月份、设置、停止条件、事件、维修实例和 JournalEntry 的读写，不包含从 Ledger 派生的现金流或报告。Application crate 的初始化、单月模拟和最终报告用例已实现；报告从已保存的月份与 JournalEntry 重建 Ledger，再调用 Core Reporting 派生财务位置、期间损益和累计现金流。Month 0-only 报告只包含期初财务位置。Application 内存适配器及用例验证仍待完成，不扩展到 schema 外的持久化事实。
- UI 用例包括设置初始值、推进一个月、停止模拟、获取统计报告和读取指定月份结果。报告从已保存的月度事实与 Ledger 派生，不触发重算。

## Application/Data Access 验收

- 用可替换的内存适配器验证 Application 的读取、随机输入、计算、停止和保存顺序。
- 用领域测试覆盖 Slice 的输入输出、不变量、来源追踪、随机边界、复式交易及生命周期规则，不依赖 IO。
- 用 SQLite 边界验证序列化、重载和 Ledger/现金流分类明细一致性。
- 检查 GPUI UI → GPUI Kit、desktop 入口 → UI/Application/SQLite Data Access/Core 的依赖方向；Core 不依赖 IO、SQLite 或 Presentation。
- 运行 `./scripts/check.sh` 和 `git diff --check`，只报告实际执行的验证。
