# Ledger and mortgage vectors

These values specify domain behavior and remain usable by future tests. They are behavioral examples, not implementation status reports.

## 月度时序与实例来源

| ID | 输入/过程 | 必须结果 |
|---|---|---|
| T-01 月份归属 | 月初冻结本月输入；Slice 在本月判定并输出事件与账务提案；月末统一结算 | 事件、效果、交易和现金流均归属当前 `Month`；月末结果成为下一月的显式期初状态；不在月末重跑普通 Slice。 |
| T-02 维修费用延迟 | 维修发生 Slice 在 Month 0 输出 RepairOccurred，RepairCost 延迟参数 N=2 | Application 据事件创建待结算实例；Month 0 快照不含该费用的交易或现金流；发生 Slice 在 Month 1、Month 2 仍正常判断；费用在 Month 2 生效结算。现金流不要求追溯到 Slice 实例或发生事件。若模拟在 Month 1 停止，不伪造 Month 2 的 Ledger 记录。 |
| T-03 维修间隔 | 模拟从 Month 0 开始，且没有已知维修费用事实 | Month 0 的间隔年龄为 0，Month 1 为 1；如 Month 1 发生维修，发生月不重置间隔，之后仍从最近一次已结算维修费用月份（此时仍为起始月）累计。若该 RepairCost 于 Month 3 结算，因当月输入已在月初冻结，Month 3 的发生判断仍使用月初 snapshot 中的旧基准；结算后的 snapshot 将 Month 3 记为新基准，Month 4 的间隔年龄为 1。结算月仍只由费用实例自身延迟参数和来源事件月份决定。 |
| T-03a 维修条件发生率 | 年龄 `a` 以完整月份计，单月率为 `(25 + floor(125 × a / (a + 60))) / 10,000` | 年龄 0、1、60、极大值的分子分别为 25、27、87、149；随年龄单调不减且始终低于 10,000。 |
| T-03b 零额维修结算 | 有效月到期的 RepairCost 从含端点分布抽得 0 | 仍输出金额为零的 `RepairCostSettled` 事件和更新最近结算月的 Effect；不生成零金额 JournalEntry，实例仍完成且不得重复计算。 |
| T-05 普通现金费用 | 本月发生现金费用 800，现金账户为资产，费用账户为费用 | 费用 800 Dr、现金 800 Cr；交易平衡，经营现金流 −800；不产生资产/负债资本化或期末结转交易。 |

## Ledger

Normal balances: assets/expenses debit; liabilities/equity/income credit. Amounts below are CAD.

| ID | Input/postings | Required result |
|---|---|---|
| L-01 Opening facts | Property 300,000 Dr; cash 20,000 Dr; mortgage 200,000 Cr; opening equity 120,000 Cr | Opening balances balance at 320,000; no period transaction or cash flow is created. |
| L-02 Mortgage payment | Mortgage 1,000 Dr; interest expense 500 Dr; cash 1,500 Cr | Balanced; principal reduces liability by 1,000. |
| L-03 Repair | Repairs expense 800 Dr; cash 800 Cr | Balanced; operating cash flow −800. |
| L-04 Rent | Cash 2,000 Dr; rental income 2,000 Cr | Balanced; operating cash flow +2,000. |
| L-05 Property tax | Property-tax expense 300 Dr; cash 300 Cr | Balanced; operating cash flow −300. |
| L-06 Projection | Apply L-02 through L-05 to L-01 | Cash 19,400; operating flow +400; financing flow −1,000; mortgage 199,000; net income 400; assets 319,400 = liabilities 199,000 + equity 120,000 + income 400. |
| L-07 Accrual capability | Interest expense 100 Dr; cash 50 Cr; interest payable 50 Cr | Balanced; operating cash flow −50. This is a generic Ledger capability, not the mortgage convention. |
| L-08 Negative cash position | Opening cash 10; month 1 repair 15; month 2 rent 3 | Signed closing cash −5 then −2; no implicit borrowing; cumulative operating flow/net income −12. |
| L-09 Detail projection | Rent +100 operating, repair −30 operating, +5/−5 investing in one transaction, financing +10 | Return transaction/category signed amounts with transaction IDs and optional source; include both range endpoints, exclude opening cash, and reconcile to category totals. |

An unbalanced opening set or transaction, duplicate/unknown account, mixed currency, invalid account role, negative posting amount or checked arithmetic overflow is rejected. Negative net worth is valid when opening facts balance.

## Fixed-rate amortizing mortgage

For opening principal `B`, nominal annual rate `r`, monthly rate `j = r / 12`, and remaining term `n`, scheduled payment is `P = B × j / (1 - (1 + j)^(-n))`; at zero rate, `P = B / n`. Round scheduled payment and monthly interest to cents half-up. Interest accrues on opening monthly principal. Payment occurs at month end, pays interest before principal; the final payment is adjusted to clear the balance.

| ID | Opening principal | Annual rate | Term | Payment | Interest | Principal | Closing balance |
|---|---:|---:|---:|---:|---:|---:|---:|
| M-01 | 1,000.00 | 0% | 10 | 100.00 | 0.00 | 100.00 | 900.00 |
| M-02 | 10,000.00 | 12% | 12 | 888.49 | 100.00 | 788.49 | 9,211.51 |
| M-03 | 100.00 | 12% | 1 | 101.00 | 1.00 | 100.00 | 0.00 |
| M-04 | 0.50 | 12% | 1 | 0.51 | 0.01 | 0.50 | 0.00 |
| M-05 | 0.01 | 6% | 1 | 0.01 | 0.00 | 0.01 | 0.00 |
| M-06 | 9,211.51 | 12% | 11 remaining | 888.49 | 92.12 | 796.37 | 8,415.14 |
| M-07 | 879.67 | 12% | 1 remaining, continuing M-02 schedule | 888.49 scheduled; 888.47 actual | 8.80 | 879.67 | 0.00 |

Reject an active loan with non-positive principal, non-positive remaining term, negative rate, or a derived scheduled payment that rounds to zero. A zero-rate loan and an absent or paid-off loan are valid. Each loan's closing balance must reconcile to its own liability account; principal is financing cash flow and paid interest is operating cash flow.
