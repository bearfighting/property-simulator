# 工程规范与质量门槛

> 本文保留原规范章节编号，便于和既有讨论对应。首版实施基线以本文及 [实施路线](implementation-roadmap.md) 为准。

### 20.10 代码组织与质量门槛

分层和低耦合是每个里程碑的验收条件，不依赖现在就确定最终 crate、目录或模块名称。每个模块在实现前/代码评审时应能回答：它负责哪一种变化、属于哪一层、依赖哪些更稳定的接口、如何独立验证。模块边界按职责与变化原因划分，避免把不同层的代码仅因同属一个 Slice 就混写；也避免为了形式把简单逻辑切成过多微小模块。

**建议的职责层次（名称与物理拆分可演进）：**

- **Core modules**：领域、账务、模型和单个 Slice 计算。接收显式输入并返回值，不依赖数据库、文件、GUI、CLI 或外部服务。模块可共享值类型和窄接口，但不得直接调用其他职责模块的具体实现。
- **Monthly Integration**：M6 单月计算中间层，调用底层独立模块完成期初准备、Slice 整合、月末结算和跨模块审计；只接收参数并返回结果，不访问 IO、不持有跨月运行状态，也不负责重复调用和停止条件。
- **Application**：最高层用例编排。它从 Data Access 读取输入/期初状态/Ledger，将值交给单月整合层，接收结果后再请求 Data Access 持久化。Data Access 只由 Application 调用；CLI/GUI 只调用 Application。当前不定义多场景调度或并行策略。
- **Data Access**：实现 SQLite 等项目内部存储。只有 Application 调用此层；存储具体表示和事务由后续存储阶段决定。不要求 schema 版本化、迁移或跨应用版本兼容。
- **Presentation**：CLI/GUI 处理用户设置的格式、默认值展开和语法解析，将其转换成明确的类型化输入；负责输入格式限制、结果展示并调用 Application。领域不变量和跨字段约束由 Core 校验；Presentation 不复制领域规则，也不直接访问 Data Access 或 Monthly Integration。

M6 成功时返回包含一致 Effects、Ledger 变化、事件、Ledger 派生现金流和期末状态的完整月度结果；错误调用不返回部分成功结果。M6 不持有跨月运行状态，也不定义失败恢复。

上面的箭头表示**运行时调用方向**：`Presentation → Application`，Application 分别调用 Data Access 和 Monthly Integration，后者调用 Core modules。Data Access 通过注入的窄接口由 Application 调用；Application 不绑定 SQLite 等具体存储实现，适配器实现该接口。实际 crate 的编译依赖应保持这一隔离，不让底层 Core 依赖 Application、Presentation 或 Data Access。Application 将读取到的值作为参数传给 Monthly Integration，再把返回结果交给 Data Access 保存。配置解析和存储表示在边界转换成类型化值；不引入全局可变单例或服务定位器，也不为每个类型机械套用 Repository/Factory/Manager。

**底层模块边界补充：** 模块之间可以引用共享的数据类型和稳定接口；不应直接调用另一职责模块的具体实现。确需协作时，由更高层调用方构造并注入窄接口（Rust trait/泛型/显式参数），模块只调用注入接口，不依赖具体实现。没有协作者的纯计算优先采用显式输入/输出函数，不为形式引入 trait。M5 完成独立模块，M6 才在中间层进行单月组合；该中间层不替代 Application，也不承担持久化或运行调度。

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

- Unit tests 验证 domain 数学、值对象、不变量和错误边界，不需 IO；随所属 Rust crate 放置，靠近被测模块（例如 `#[cfg(test)]` 子模块），不为尚未实现的领域行为添加占位测试。
- 对已接受的输入和允许的随机结果，计算必须产生领域输出；运行时数值错误、冲突拒绝或账务校验失败属于缺陷证据。通过属性/边界测试覆盖可表示范围；输入校验测试只验证运行前/输入冻结前拒绝结构非法或无法表示的配置。
- Application tests 在该层实现后使用内存 Data Access 验证“读取→传参计算→保存”的调用次序和数据传递；M6 单月组合测试验证真实 Core 模块边界。序列化/数据库测试在 Data Access 实现后增加。早期 M3/M4 完整运行流程只保留在 simulation test support，不能当作当前生产架构。
- Ledger 与 mortgage 共享手算预期值维护在 [`test-vectors.md`](test-vectors.md)，由对应测试引用；规则变化时同步更新关联 ADR、领域说明和向量，避免重复维护。
- Property/invariant tests 覆盖金额/账务平衡、状态转换、随机输入边界和 ID/引用约束。固定 seed/golden fixtures 用于计算回归；不同调用之间不要求重放相同随机轨迹。
- 测试名称表达领域行为；失败信息包含输入场景和预期/实际值。测试不得依赖当前时间、外部网络、未固定 seed 或执行顺序。
- 每个里程碑的验收记录格式、lint、编译和所需测试结果；具体要运行哪些命令由项目脚本定义。代码评审须检查层级职责、依赖方向、持久化/配置/展示是否越界，以及是否引入了没有现实需求支撑的抽象。

**变更验收清单：**

1. 领域规则是否留在 Domain，存储/配置/展示是否经边界转换？
2. 模块依赖方向是否符合约定，是否出现反向依赖、跨层直接访问或重复业务逻辑？
3. 成功月度结果是否包含一致的相关事实？fatal 失败是否停止模拟且不产生失败月份成功结果？Core 是否避免引入历史归档或持久化恢复责任？
4. 对应层级的单元/集成测试是否覆盖行为和失败路径？
5. 格式、lint、编译与相关测试是否通过？若未执行，是否明确记录原因？
6. 新增抽象/设计模式是否解决了已出现的耦合或变化点，而非只为预想的未来复杂度？
