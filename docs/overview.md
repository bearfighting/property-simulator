# 项目概览

Property Simulation Sandbox 是面向个人/家庭房产持有者的可配置、可审计月度模拟器。它计算用户提供的期初事实和未来假设在逐月变化中的影响；模拟结果是模型输出，不是对特定房产的确定性预测、投资建议或法定会计记录。

## 当前范围

- 以一个房产和零笔或一笔独立贷款为首版主体；按月计算，不设低于 Month 可表示范围的固定模拟期限。
- 从用户提供的充分期初状态开始，不补造模拟开始前的购买、付款或其他交易。
- 保留事件、状态变化、Ledger 经济事实及其来源；现金流从 Ledger 推导。
- Application 与 Data Access 负责用户 context、逐月调用、停止条件和 SQLite 持久化；当前未完成工作见[路线图](implementation-roadmap.md)。
- 负房产现金流是有效结果。初始范围不模拟家庭偿付能力、破产、资金不足处置、逾期、违约、止赎或计算中途等待用户决策。

## 明确不做

- 投资建议、回报评分、IRR/NPV、投资组合优化或对房屋未来作确定性预测。
- 法定或税务用途的完整会计实现。
- 宏观经济均衡、精准天气预测或建筑工程级仿真。
- 云端账户、多人协作、插件市场、通用规则 DSL、分支、多场景管理或并行调度。
- 导出格式或模拟结束后的长期结果归档策略。

## 核心术语

| 术语 | 含义 |
|---|---|
| Initial state | 模拟起点已知事实与足以继续计算的状态 |
| Simulation state | Application 管理的模拟配置、随机 root 和继续模拟所需事实；各项事实独立组织，不要求形成一个通用 Context 聚合 |
| Effective monthly input | 某月计算实际使用的冻结输入 |
| Activity instance | 用户配置或模型生成、在特定月份生效的模拟对象 |
| Slice | 对活动进行计算的代码规则边界；与活动实例不要求一一对应 |
| Event | 已发生的事实 |
| Decision | 用户或策略的选择 |
| JournalEntry | 同一模拟月份的一组平衡复式分录；Month 0 用于初始化，Month 1 起属于模拟期间 |
| Monthly result | 各 Slice 对单月计算的结果；Application 按月份组织并保存需要的事件和分录 |

上下文、计算和存储边界见[领域模型](domain-model.md)、[架构](architecture.md)和[实施计划](implementation-roadmap.md)。
