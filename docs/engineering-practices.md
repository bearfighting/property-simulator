# 工程规范与质量门槛

> 本文保留原规范章节编号，便于和既有讨论对应。首版实施基线以本文及 [实施路线](implementation-roadmap.md) 为准。

### 20.10 代码组织与质量门槛

分层和低耦合是每个里程碑的验收条件，不依赖现在就确定最终 crate、目录或模块名称。每个模块在实现前/代码评审时应能回答：它负责哪一种变化、属于哪一层、依赖哪些更稳定的接口、如何独立验证。模块边界按职责与变化原因划分，避免把不同层的代码仅因同属一个 Slice 就混写；也避免为了形式把简单逻辑切成过多微小模块。

**建议的职责层次（名称与物理拆分可演进）：**

- **Domain**：实体、值对象、不变量、领域错误、利息/故障等纯计算规则；不依赖数据库、文件、GUI、CLI 或外部服务。
- **Application / Services**：用例编排、运行流程、Slice 调度、事务边界和端口调用；协调 domain，不重新实现领域规则。
- **Configuration**：场景输入、默认值展开、版本化配置、解析和校验；转换成明确的 domain/application 输入，不让配置格式类型渗入领域核心。
- **Data Access / Infrastructure**：SQLite、文件、序列化、随机数生成器适配、时钟等外部能力；通过接口由上层注入，不能反向依赖具体 domain 实现细节。
- **Unit of Work / Work Unit**：定义一次提交的工作边界。协调器在同一提交边界内校验并原子保存状态 Effects、Ledger 凭证、事件和快照；失败不留下部分提交，重复提交具备幂等语义。内存实现可先用于核心测试，持久化实现后续提供。
- **Presentation**：CLI、GPUI 及导出适配层负责输入/展示/调用应用用例，不承载领域计算或直接操作存储实现。

建议依赖方向为 `Presentation → Application → Domain`，`Application → Ports`，`Infrastructure → Ports/Domain contracts`；Domain 不向外层依赖。配置解析和存储格式通过边界转换成稳定的内部类型。依赖注入使用 Rust trait/泛型/显式构造参数等简单机制；不引入全局可变单例或服务定位器。不要为每个类型机械套用 Repository/Factory/Manager 等设计模式；仅在存在实际变化边界或需要替身测试时引入接口/模式。

**静态质量检查：**

工程骨架的统一入口为 `./scripts/check.sh`，本地开发和 GitHub Actions 均调用该脚本。脚本依次执行：

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo check --workspace --all-targets`
4. `cargo test --workspace`

Rust toolchain 使用 `stable`，并安装 `rustfmt` 与 `clippy`；workspace edition 为 2024。新增 lint warning 应修复，局部 allow 需附清楚理由。

- Rust 格式统一由 `rustfmt` 执行，CI/提交前以 `cargo fmt --all --check` 验证；不接受未格式化代码。
- Clippy 作为 lint 基线，CI 执行 workspace 范围检查；明确选定并记录 lint 级别，新增 warning 应修复或附有范围最小、理由清楚的 allow，不用全局 allow 掩盖问题。
- 编译器 warning 纳入质量门槛；依赖版本和特性保持明确，避免不必要的依赖及未使用依赖。
- 格式、lint、编译和测试命令纳入统一脚本/CI；项目骨架建立时确定命令，不要等 GUI 阶段再补。

**测试分层与验收：**

- Unit tests 验证 domain 数学、值对象、不变量和错误边界，不需 IO。
- Application/service tests 使用内存端口或 fake 验证用例编排、Slice 顺序、Unit of Work 提交/回滚和幂等行为。
- Integration tests 验证真实模块边界、序列化/反序列化、数据访问适配和完整组合流程；数据库相关测试在引入持久化后增加。
- Property/invariant tests 覆盖金额/账务平衡、状态转换、checkpoint 恢复、快照分支隔离、随机流分配和 ID/引用约束；固定 seed/golden fixtures 用于软件模型回归，不要求独立重模拟得到相同轨迹。
- 测试名称表达领域行为；失败信息包含输入场景和预期/实际值。测试不得依赖当前时间、外部网络、未固定 seed 或执行顺序。
- 每个里程碑的验收记录格式、lint、编译和所需测试结果；具体要运行哪些命令由项目脚本定义。代码评审须检查层级职责、依赖方向、持久化/配置/展示是否越界，以及是否引入了没有现实需求支撑的抽象。

**变更验收清单：**

1. 领域规则是否留在 Domain，存储/配置/展示是否经边界转换？
2. 模块依赖方向是否符合约定，是否出现反向依赖、跨层直接访问或重复业务逻辑？
3. 事务/Unit of Work 是否确保相关事实原子提交且可幂等？
4. 对应层级的单元/集成测试是否覆盖行为和失败路径？
5. 格式、lint、编译与相关测试是否通过？若未执行，是否明确记录原因？
6. 新增抽象/设计模式是否解决了已出现的耦合或变化点，而非只为预想的未来复杂度？
