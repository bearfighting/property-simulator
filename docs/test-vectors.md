# Ledger, mortgage, and historical composition test vectors

**Historical M3/M4 fixture note:** Vectors explicitly about the old M3/M4
run-control and multi-slice fixtures describe behavior retained in
`crates/simulation/src/test_support/` and test modules. They are regression coverage,
not production run-control, snapshot, branch, or persistence API promises.
Ledger, mortgage, and independent module vectors remain relevant domain or
module-level coverage. Current M5/M6 boundaries are defined by ADR-27 through
ADR-29. Some vectors name the former
`CashFlowContribution` implementation. Those values are expected cash-flow
projections from Ledger postings, grouped by the Ledger source identity. They do
not require a second independently generated contribution record. M5 uses
module-level vectors; full monthly composition vectors belong to M6 under
[ADR-27](adr/ADR-27-current-month-runtime-and-database-owned-history.md) and
[ADR-29](adr/ADR-29-layered-calculation-and-application-boundaries.md).

## M3.4 semantic random-stream vectors

These vectors verify the existing randomness adapter's deterministic stream
isolation for tests and diagnostics. The `trial_id` used in those identities is
not a persisted application run or branch requirement.

With diagnostic root key `[8; 32]`, calls using the same root and trial derive the same run key; a different trial derives a different key. The current tests construct semantic streams directly from explicit identity; run-control integration is test-only. A stream collection rejects a semantic identity for a different trial. Under one run key, identical semantic identity yields the same sequence. Changing category, Slice ID, activity instance ID, year/month, or purpose yields an independently derived stream. Consuming another purpose stream does not affect it. OS entropy success fills all 32 bytes; entropy failure is returned. Purpose is UTF-8 with an explicit `u64` byte length. These are implementation-isolation vectors, not replay guarantees or production run-management behavior.

## M3.5 historical repair random end-to-end vector

This vector records the former integrated test path through `MonthlyResult` and
Ledger posting. It documents test coverage, not a current production result type
or a separate cash-flow contribution record.

The test caller derives a `Property` semantic stream from trial ID,
Slice ID, activity instance ID, and period with purpose `repair.sample`.
With diagnostic root key `[8; 32]` and the same semantic identity, the typed repair
sample is repeatable within the same run streams. The resulting `ActivitySample`
retains that purpose through `propose_slice` and `MonthlyResult`; explicit caller
samples retain `purpose = None`. `RepairSlice` receives only the typed sample and
still validates its cost against the frozen repair support before proposing any
effect or posting. The committed repair transaction, per-instance cash-flow
contribution, and Ledger direct cash-flow projection agree. Repeated values are
diagnostic behavior for this fixed key, not a cross-application-version replay guarantee.

## M3.7 composition acceptance vectors

**Slice 代数组合约定：** 每个实例只根据同一份期初状态、冻结输入及自身显式样本计算，不消费其他实例的当月输出。对可接受实例结果做任意排列或重新分组，组合结果必须相同；空结果是单位元。相同状态值可兼容合并，冲突值须在提交前拒绝；账簿凭证保留独立来源并作为批次验证。读写资源交叠不建立执行顺序。依赖字段、拓扑阶段或跨实例结果上下文均不属于当前 Slice 契约。

These vectors are exercised across test-only M3 run/executor/coordinator fixtures,
Slice outputs, Ledger, and loan tests. They do not assert that the current
production core exposes that integrated workflow.

| Case | Expected acceptance |
|---|---|
| Inclusive two-month run ending at February | Each `Continue` commits exactly one month. The February snapshot is retained before end/condition evaluation; status reports the configured stop reason. A between-month `Stop` preserves the latest completed snapshot and does not invoke the executor. |
| Month-start input update | A valid update effective in the next period is validated and included in the completed result with its source and effective month. Invalid user input is rejected before calculation so it can be corrected and resubmitted. A fatal error during an accepted transition stops the simulation and produces no result for that month. |
| Balanced month-start state adjustment | Its transaction is retained in the completed result with the update source and transaction ID. A separate run-input Effect records prior/updated state and links to that transaction. Cash entries produce input cash-flow contributions that aggregate with Slice contributions and reconcile to the period Ledger projection; no fabricated Slice/activity identity is used. |
| Fatal monthly transition | A fatal executor/Slice transition stops the simulation and returns a diagnostic carrying phase and month. The failed month has no successful result or snapshot and is not reported as a normal stop. No failed-run recovery or persistence promise is made. |
| Repair + rent + two loans in one period | A single `SimulationRun` test executor draws a semantic repair sample, proposes rent and repair outputs, and calls the real loan coordinator for two independent loans. Every transaction is balanced and source-linked to its event/activity. Events, Effects, samples, transactions and contributions carry the same trial, branch, month, Slice and activity identity; output IDs are unique. Rent/repair contributions and loan interest/principal contributions sum by category to the Ledger direct cash-flow projection. Each loan continuation equals its own liability account balance. |
| Two repair instances superpose | Two repair instances evaluate from the same opening state and frozen inputs with separate samples. Their compatible condition Effects combine idempotently, their postings and contributions remain separate and attributable, and reversing instance/sample order produces the same `MonthlyResult` and Ledger. |
| Mixed Slice final-balance boundary | A loan principal advance and its monthly payment are prepared against a cash balance within 500 minor units of the maximum `Money` coefficient. The loan batch alone would overflow, while an ordinary repair outflow offsets the advance and payment so the full month closes at a representable cash balance. The shared coordinator accepts the month and the contribution projection still matches the Ledger. |
| Random semantic streams across periods and activities | Repair samples carry `repair.sample` and are allocated from the explicit Property semantic identity. Different period/activity/purpose identities are isolated; consuming another category's stream does not affect them. Fixed diagnostic keys test isolation, not cross-run replay. |
| Branch from a completed snapshot | A new trial and branch begin in the month after the selected snapshot, restore its closing state, Ledger, inputs, activities and loan continuations, and may accept explicit updates. Saving child months leaves parent snapshots/results unchanged. Child cash-flow summaries include child months only; the month sequence is contiguous within each branch. |

The shared assertions are implemented in test-support lifecycle, failure,
stop and composition tests; `crates/simulation/src/monthly_composition_tests.rs` joins a two-period run, semantic
repair sampling, rent/repair Slices, two loan accounts, Ledger projections, a
month-start rent update and a child branch; and lower-level tests cover the
loan-update and independent-stream boundaries. This matrix intentionally does
not claim an unfinished-run checkpoint or persistent rollback guarantee.

## M4.2 operating activity vectors

These vectors record the accepted deterministic and stochastic activity rules.
Amounts are CAD. Expense debits and cash credits are balanced operating
transactions; cash-flow projections are negative outflows. Zero economic-output
cases are the empty-proposal identity; the deterministic tax input marker is
retained separately for provenance. Rows that require a shared coordinator,
committed Ledger batch, or complete monthly result describe historical test-only
composition; the activity calculation rows remain module-level expectations.
Utility
usage is an integer count in the activity instance's configured unit; rate
multiplication uses checked fixed-point arithmetic and rounds once at posting.

| Case | Inputs | Expected result |
|---|---|---|
| Property tax monthly due | Amount $100.00, monthly cadence, first due 2026-01; period 2026-02 | Monthly result retains the deterministic `PropertyTax` marker; Dr property-tax expense $100.00; Cr cash $100.00; one event with key `property_tax.payment` and one transaction with output key `payment`; the transaction's `source_event_key` is `property_tax.payment` for that same instance and period, and the -$100.00 operating contribution references the transaction ID |
| Property tax quarterly schedule | Amount $300.00, quarterly cadence, first due 2026-01; periods 2026-02 through 2026-04 | February and March retain only the deterministic input marker and have no economic proposal outputs; April posts exactly $300.00. January is not re-created if the run starts in February. No monthly accrual is posted. |
| Property tax annual boundary | Amount $1,200.00, annual cadence, first due 2026-12; periods 2026-12, 2027-01, and 2027-12 | December 2026 posts $1,200.00; January 2027 retains only the deterministic input marker and has no economic proposal outputs; December 2027 posts $1,200.00. The first due month anchors the annual schedule across the year boundary. |
| Zero due amount | Amount $0.00 on a due month | The deterministic `PropertyTax` marker is retained; no event, Effect, transaction, or cash-flow contribution is emitted, so economic outputs equal the empty identity. |
| Rounded-zero payment | Amount $0.004 on a due month | The deterministic `PropertyTax` marker is retained. Posting rounds to $0.00; no zero-value transaction, event, Effect, or contribution is emitted. |
| Invalid operating account role | Property-tax or utilities expense account is marked contra; alternatively, configured cash account is not a non-contra cash-position asset | Reject the activity configuration before transition; do not post a transaction or emit economic outputs. |
| Utility positive usage | The utility model accepts a sample of 12 units within the configured maximum of 100; rate $2.50/unit | Calculate a $30.00 charge; Dr utilities expense $30.00; Cr cash $30.00; one event with key `utility.usage_charged` and one transaction with output key `usage-charge`; the transaction's `source_event_key` is `utility.usage_charged` for that same instance and period. The Ledger-derived operating cash-flow projection is -$30.00 and references the transaction ID. Whether M6 includes the sampled value in its returned result belongs to the M6 result contract. |
| Utility uniform sampler | Maximum usage 7; uniform inclusive integer sampler receives diagnostic words 0 and 7 | Samples 0 and 7 respectively; output always lies in `0..=7`, and a maximum of 0 always yields 0. Semantic production sampling fixes category `occupancy/operations` and purpose `utility.usage`. |
| Utility zero usage | Maximum 100 units, sample 0, rate $2.50/unit | The sampled value is retained; no event, Effect, transaction, or contribution is emitted, so economic outputs equal the empty-proposal identity. |
| Utility minor-unit rounding | Sample 3 units, rate $0.125/unit | Exact charge $0.375 rounds once to $0.38 half-up; expense and cash postings are each $0.38 and the transaction balances. |
| Utility rounded-zero charge | Sample 3 units, rate $0.001/unit | Exact charge $0.003 rounds to $0.00; the sample is retained but no zero-value transaction, event, Effect, or contribution is emitted. |
| Utility support uses posted rounding | Maximum usage 100 units, rate $0.00004/unit | Validate that the exact maximum product $0.004 fits `Money`; its rounded maximum posted charge and operating cash-flow support contribution are both $0.00. Aggregate support uses the rounded posted bound, not the exact pre-rounding product. |
| Utility support boundary | Maximum 100 units; samples 0 and 100 | Both samples are accepted; the resulting charges are respectively zero (empty economic proposal) and `100 × rate`. A sample above 100 or with mismatched carried period/Slice/activity/model/purpose is rejected before commit. The utility sampling boundary always uses `occupancy/operations` and `utility.usage`; the historical test-only `RandomStreams` adapter rejects a mismatched trial identity, while another category derives an independent stream under ADR-21 rather than being generically rejected. |
| Utility unrepresentable support | Maximum usage and rate whose exact product is outside the supported `Money` range, or whose maximum operating support makes the whole-period category bound unrepresentable when combined with other active contributors | Reject the activity/run input before sampling. If calculation overflows despite accepted prevalidation, fail the transition as fatal under ADR-19. |
| Utility pre-sampling boundary | A utility sample is requested from `ValidatedInput` before the complete active activity set has been validated | Reject sampling/evaluation. After `validate_activities_before_sampling` validates every active activity and the combined category support, samples for the same frozen month and exact activity instances are accepted. |
| Exact utility product reduction | Rate is `Money::from_parts(i128::MAX, 1, CAD)`, maximum usage is 10 | Exact product is representable as `Money::from_parts(i128::MAX, 0, CAD)`; do not reject due only to an overflowing unreduced intermediate coefficient. |
| Aggregate operating support boundary | A due property-tax payment and rounded maximum utility charge each fit in `Money`, but their absolute support plus other active operating contributors exceeds the period category bound | Reject the monthly input before transition (and before utility sampling); support sums use maximum posted amounts after currency minor-unit rounding, and reversing Slice order cannot turn an unrepresentable support into an accepted month. |
| Shared cash and permutation | One due tax instance for $100.00 and one utility instance for 12 units at $2.50/unit, both using the same cash account | Separate balanced transactions and unique source-linked outputs are retained; reversing instance/result order yields the same committed Ledger and aggregate operating outflow -$130.00. |
| Grouping across new and existing activities | One due tax instance for $100.00, two utility instances for $20.00 and $30.00, and one Rental instance receiving $80.00; all evaluate independently against the same opening state and frozen month | Merge the four evaluated instance proposals as `((tax ⊕ utility-1) ⊕ (utility-2 ⊕ rent))` or `(tax ⊕ (utility-1 ⊕ utility-2)) ⊕ rent`, then pass each merged proposal set through the same monthly coordinator once. Both paths yield the same canonical monthly result, Ledger, retained output identities, and net operating contribution -$70.00. Every positive posting's transaction source-event key matches its same-instance, same-period event, and its contribution references that transaction ID; transactions remain separate. |
| Mixed activity Ledger reconciliation | The preceding tax and utility transactions are batched with rent, repair, and loan transactions | All transactions remain independently attributable and balanced; summed signed activity contributions equal the Ledger direct operating projection for their operating postings. No activity reads another activity's result or account balance. |
| Property tax with existing rent/repair/loan composition | A $100 due tax payment, $80 rent, and $10 repair share the cash account; separately, a due tax payment shares cash and the interest expense account with a loan payment | Each activity retains its own balanced transaction and source identity. Reversing tax/rent/repair proposal and sample order preserves the result and Ledger. The loan composition retains tax and loan contributions together, including separate output provenance. A month with no activity instances coordinates successfully with an empty result and unchanged Ledger. |

For both activities, trial/branch, period, Slice, activity ID,
output kind, and stable output key determine provenance and output identity under
ADR-20. Reusing an activity/output identity is rejected before commit. Neither
activity writes an Effect or changes opening world state.

These hand-calculated vectors are the shared initial verification baseline for Ledger and mortgage calculations. Amounts are CAD unless stated; all values are exact dollars. Posted values use cents and positive half-up rounding. `Dr` and `Cr` mean debit and credit. A transaction balances when debits equal credits. ADRs reference IDs here to avoid duplicated arithmetic.

## M2 monthly activity vectors

### M3.3 historical cash-flow contribution vector

The `CashFlowContribution` wording below reflects the former composition fixture;
the current contract is the equivalent cash-flow projection derived from Ledger
postings and source metadata.

For a month with rent of CAD 80.00 and repair cash cost of CAD 10.00, the result retains two operating contributions: rental transaction +80.00 and repair transaction -10.00. Each item carries its posted transaction ID and source identity (trial, branch, activity instance, Slice, output key); monthly operating total is +70.00 and matches the Ledger direct cash-flow projection. A non-cash transaction has no contribution. Within one transaction, cash entries are netted per category; categories remain distinct.

Before sampling, sum each active Rental instance's rounded rent ceiling and each Repair instance's rounded repair ceiling as operating support; add each loan's calculated interest support as operating and principal support as financing (including an effective principal update); then add each month-start state-adjustment transaction's net cash contribution by category. If any category's absolute support sum exceeds `Money`, reject the month input before sampling. Month result construction also rejects actual per-instance Rental/Repair contributions beyond their support; loan coordination checks resulting category totals for direct callers. These checks keep monthly partial sums representable under any order or grouping. Archive aggregation accumulates exact signed decimals and returns an explicit arithmetic error only if the final saved-run category total is outside the `Money` range. A child run summary includes its own saved months only, beginning after its parent snapshot.

The run-control boundary regression combines a one-month, zero-rate loan with a near-maximum financing `StateAdjustment`: each transaction and balance is individually representable, but their financing contribution sum is not. The update is rejected before either executor validation hook or monthly execution; run status, history, Ledger, and snapshots remain unchanged.

All amounts below are CAD and opening cash is a signed cash-position balance. Rental debits
cash and credits rental income, classified as operating cash flow. Repair debits operating
expense and credits cash. A repair event links to its condition Effect and balanced transaction;
the condition Effect is non-cash. Transactions balance independently. Rental receipts likewise
retain an event-to-transaction source link.

| Case | Opening state | Frozen input / sample | Expected result |
|---|---|---|---|
| No repair | cash 100, NeedsRepair | rent 80, limit 50, NoRepair | cash 180; rental +80 operating; no repair transaction/effect |
| Minor-unit rounding | cash 0, NeedsRepair | rent 80.005, limit 50.005, Repair(1.00) | rent posts 80.01; repair expense/cash post 1.00; both transactions balance |
| Repair lower bound | cash 0, NeedsRepair | rent 80, limit 50, Repair(0) | cash 80; condition Available; zero-cost balanced transaction |
| Preventive repair | cash 10, Available | rent 0, limit 50, Repair(5) | cash 5; expense 5; condition remains Available |
| Repair upper bound | cash 10, NeedsRepair | rent 80, limit 50, Repair(50) | cash 40; expense 50, rental 80; operating net +30; Available |
| Negative cash remains valid | cash 0, NeedsRepair | rent 0, limit 50, Repair(50) | cash -50; expense 50; operating cash flow -50; Available |
| Above support rejected | cash 10, NeedsRepair | limit 50, Repair(50.01) | injected out-of-support sample rejected by the Slice |

Repair outcomes range from zero through the largest supported minor-unit amount; `NoRepair` is a
separate outcome. The configured distribution maximum must match the frozen input limit and is validated
before sampling, including whether its integer minor-unit support fits the sampler's `u64` range.
Repair outcomes are discrete integer minor-unit values through `floor(maximum / minor_unit)`;
fractional-minor-unit samples and a fractional upper-bound tail are rejected. This bounded
example does not admit an unbounded distribution. Positive rent and repair amounts are rounded
to the currency minor unit using half-up before proposal entries are created; this keeps every
accepted proposal postable by the Ledger.

The M2.3 repair law uses an exact rational occurrence probability. Denominator zero and
numerator greater than denominator are invalid; numerator zero and numerator equal to denominator
represent the probability boundaries. Conditional cost is uniform over integer minor units from
zero through `floor(maximum / minor_unit)`. For CAD, a maximum of 1.239 yields support 0.00 through
1.23; a maximum below 0.01 yields the single supported value 0.00.

## Ledger vectors

The minimal accounts used below are Cash (asset), Property (asset), Mortgage payable (liability), Opening equity (equity), Rental income (income), Interest expense, Repairs expense, and Property tax expense (expenses). Asset/expense normal balances are debit; liability/equity/income normal balances are credit.

| ID | Transaction | Debit | Credit | Check |
|---|---|---|---|---|
| L-01 | Opening balances: property $300,000, cash $20,000, mortgage $200,000; opening net worth $120,000 | Property 300,000; Cash 20,000 | Mortgage 200,000; Opening equity 120,000 | 320,000 = 320,000; opening-balance record, not a simulated-period transaction or cash flow |
| L-02 | Monthly payment $1,500: principal $1,000, interest $500 | Mortgage 1,000; Interest expense 500 | Cash 1,500 | 1,500 = 1,500; loan balance falls by 1,000 |
| L-03 | Repair paid $800 | Repairs expense 800 | Cash 800 | 800 = 800 |
| L-04 | Rent received $2,000 | Cash 2,000 | Rental income 2,000 | 2,000 = 2,000 |
| L-05 | Property tax paid $300 | Property tax expense 300 | Cash 300 | 300 = 300 |
| L-06 | Reconciliation after L-02 to L-05 | Opening balances plus listed debits/credits | — | Cash = 20,000 - 1,500 - 800 + 2,000 - 300 = 19,400; operating cash flow = -500 - 800 + 2,000 - 300 = 400; financing cash flow = -1,000; net cash change = -600; mortgage = 199,000; net income = 400; assets 319,400 = liabilities 199,000 + opening equity 120,000 + net income 400 |
| L-07 | Generic accrual edge: interest expense $100; cash interest paid $50; remaining accrued amount $50 credited to an interest payable | Interest expense 100 | Cash 50; Interest payable 50 | Balanced; $50 operating cash outflow; liability rises by $50; no cash-flow line for the liability credit. Ledger projection capability only; not an outcome of the ADR-14 mortgage convention |

M1.3 opening-balance checks: L-01 is represented by four opening facts (property debit $300,000; cash debit $20,000; mortgage credit $200,000; equity credit $120,000), with derived totals of assets $320,000, liabilities $200,000, equity/net worth $120,000. The opening facts remain outside period transactions and cash flow. A negative-net-worth case uses assets debit $80, liabilities credit $100, and equity debit $20; the debit equity balance is valid and the opening set still balances. A normal asset credit balance is rejected; a contra-asset credit balance reduces net assets. Reject an unbalanced set, negative amount, duplicate or unknown account, income/expense account, mixed currency, or declared net worth that differs from assets less liabilities.

### L-08 — Negative cash-position continuity

| Period activity | Debit | Credit | Check |
|---|---|---|---|
| Month 1: repairs $15 | Repairs expense 15 | Cash position 15 | Balanced |
| Month 2: rent received $3 | Cash position 3 | Rental income 3 | Balanced |

Opening cash position is $10 Dr. Month 1 closes at cash position $5 Cr (signed −$5), not a generated overdraft/loan; month 2 closes at $2 Cr (signed −$2). Cumulative property operating cash flow is −$12; cumulative net income is −$12; modeled assets are signed cash −$2; liabilities $0; equity −$2. Both month-end states reconcile and simulation continues.

L-08 is a scoped property-ledger result. The negative cash position does not assert that a real bank paid the shortfall, that the household borrowed, or that any off-ledger household funding occurred.

### Money range vectors

Money uses a signed `i128` coefficient and scale up to 28. Parsing a 38-digit integer and adding it to another amount must remain exact when the result fits; adding one to `i128::MAX` must return an explicit arithmetic overflow. A value with scale 29 is rejected. No accepted or calculated amount may wrap or be clamped.

## Mortgage vectors

For active fixed-rate, fully amortizing loans, derive the level payment from opening principal, remaining months and nominal annual rate using the annuity formula in ADR-14. The effective month is the first full interest month and first payment month; the first payment is made at that month's close. Round the scheduled payment and each month's interest to cents half-up. The scheduled payment stays constant; the final payment is adjusted to the remaining principal plus that month's interest. Values below are CAD.

| ID | Opening balance | Annual rate | Term (months) | Scheduled payment | Period | Interest | Actual payment | Principal paid | Closing balance |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| M-01 Zero rate | 1,000.00 | 0% | 10 | 100.00 | 1 | 0.00 | 100.00 | 100.00 | 900.00 |
| M-02 Ordinary amortization | 10,000.00 | 12% | 12 | 888.49 | 1 | 100.00 | 888.49 | 788.49 | 9,211.51 |
| M-03 One-month payoff | 100.00 | 12% | 1 | 101.00 | 1 (final) | 1.00 | 101.00 | 100.00 | 0.00 |
| M-04 Monthly payment rounding | 0.50 | 12% | 1 | 0.51 | 1 (final) | 0.01 | 0.51 | 0.50 | 0.00 |
| M-05 Interest below half-cent | 0.01 | 6% | 1 | 0.01 | 1 (final) | 0.00 | 0.01 | 0.01 | 0.00 |
| M-06 Continuation month | 9,211.51 | 12% | 11 remaining | 888.49 | 2 | 92.12 | 888.49 | 796.37 | 8,415.14 |
| M-07 Final-payment adjustment | 879.67 | 12% | 1 remaining | 888.49 (M-02 schedule) | 12 (final) | 8.80 | 888.47 | 879.67 | 0.00 |
| M-08 No active loan | 0.00 / paid off | — | — | — | — | 0.00 | 0.00 | 0.00 | 0.00 |
| M-09 Zero term rejected | 1,000.00 | 12% | 0 | — | — | — | — | — | Reject before transition |
| M-10 Negative rate rejected | 1,000.00 | -1% | 12 | — | — | — | — | — | Reject before transition |
| M-11 Non-positive active balance rejected | 0.00 | 12% | 12 | — | — | — | — | — | Reject before transition |
| M-12 Derived payment rounds to zero | 0.01 | 0% | 1,000 | 0.00 | — | — | — | — | Reject before transition |

M-02's unrounded annuity payment is approximately $888.4878868 and therefore posts at $888.49. M-06 continues that schedule from the first closing balance; its payment remains unchanged. M-07 is the twelfth payment of the M-02 schedule after eleven regular payments: $888.49 is the scheduled payment carried forward from that schedule, not a new payment derived from the displayed final-month balance and one remaining period. The actual payment is adjusted down to settle the remaining balance exactly after monthly interest rounding. M-04 tests a half-cent midpoint: $0.50 × 1% = $0.005 → $0.01 interest, while the formula-derived payment $0.505 rounds to $0.51. M-05 tests a positive balance whose monthly interest rounds to zero.
M-02 period 1 is the effective month and includes both the full month's interest and its first month-end payment.

### Mortgage Ledger posting cross-checks

These M3 cross-checks apply the mortgage outputs above to the Ledger. Opening principal is an opening liability balance, not a past-period transaction. New simulated borrowing posts cash received against the loan liability as financing inflow. For an active payment month, debit interest expense by interest and debit the loan liability by principal paid; credit cash for interest as operating flow and principal as financing flow. Each loan continuation balance equals its own Ledger liability; all loan interest uses the shared interest expense account.

| Case | Mortgage output | Balanced Ledger posting | Closing check |
|---|---|---|---|
| M-02 first payment | Interest 100.00; payment 888.49; principal 788.49 | Dr Interest expense 100.00; Dr Mortgage liability 788.49; Cr Cash (operating) 100.00; Cr Cash (financing) 788.49 | Liability falls from 10,000.00 to 9,211.51; cash falls 888.49 |
| M-03 final payoff | Interest 1.00; payment 101.00; principal 100.00 | Dr Interest expense 1.00; Dr Mortgage liability 100.00; Cr Cash (operating) 1.00; Cr Cash (financing) 100.00 | Liability falls from 100.00 to 0.00; cash falls 101.00 |

The cash postings may share one cash account but retain their separate cash-flow classifications. Each transaction balances, and each loan instance balance reconciles to its own liability account; interest expense is shared.

### M3.6 loan Slice boundary

`LoanSlice::propose_month` is a pure transformation from a typed monthly calculation to a loan proposal. The historical test-only loan coordinator validates account roles and opening liability, prepares and posts a balanced transaction, and checks the closing liability. `coordinate_month_with_loans` and the resulting `MonthlyResult` are test-only; the production core does not commit a Ledger batch or retain loan continuation history. The vectors below continue to check interest/principal classifications and loan arithmetic, not a current integrated API.

The model tests implement M-01 through M-07 and M-09 through M-12; M-08 is the absence of an active loan and produces no payment proposal. Continuation keeps the rounded payment from M-02 through M-06; explicit rate/term/balance inputs clear and rederive that payment before calculation. A carried payment with the wrong currency, subminor precision, zero/negative value, or below current posted interest is rejected. Principal outside currency minor-unit precision is rejected at model validation. Negative cash is allowed by the cash-position account convention and does not alter loan calculation. Historical run and branch continuation cases remain only in test support. The monthly composition fixture at `crates/simulation/src/monthly_composition_tests.rs` exercises those prior combinations; it is not a production API contract.

## M4.5 unified monthly composition vector

At `YearMonth(u32::MAX, 12)`, coordinate rent, two repair instances, a due property-tax payment, two utility instances, and two active fixed-rate loans. The ordinary activities share one cash-position account; property tax and repairs use one expense account, while utilities and loan interest use another; each loan has its own liability account. Both repairs propose the same resulting property condition, which the Effect reducer accepts. All activity transactions remain separate, balanced, and linked to their event and source instance. Each loan continuation reconciles to its own posted liability balance. The per-category sum of activity cash-flow contributions equals the Ledger direct cash-flow projection. Reversing ordinary output/sample order and loan input order yields the same monthly result and Ledger, including transaction history order.

## M4.6 multi-month operating lifecycle vector

The test-owned monthly executor composes rent, repair, property tax, utility, and two
loans through the same coordinator and Ledger batch for January and February. January
uses tax amount $25 and each utility instance's initial rate. A February
`RunUpdateValue::Activities` replaces the tax amount with $35 and changes the utility
rates; January's result and snapshot retain the original configurations and February
uses the replacement configurations. Utility samples are drawn independently from
the `occupancy/operations` semantic identity for each activity instance and retain
their `utility.usage` purpose, instance, Slice and period. The run
stops after the inclusive February boundary and the February snapshot retains its
samples, configurations, Ledger, and cash-flow contributions.

Forking the January snapshot creates a new trial and branch identity. The child
applies a distinct February activity configuration while the parent continues with
its own February update. Both branches begin February from the January closing
snapshot; their tax/utility postings, loan balances, snapshots, and configuration
history remain isolated, and the parent January snapshot stays byte-for-byte
unchanged. A fixed diagnostic key runs the same February composition with reversed
activity and loan input order; compare the successful monthly outputs, Ledger, and
contributions. Compare each random sample under its semantic identity;
equal numeric samples across different instances are allowed.

## M4.7 historical failure-boundary vectors

| Boundary | Expected result |
|---|---|
| Correctable user input rejected before calculation | Reject the input before starting the monthly calculation. The user may correct it and submit the command again. This is input validation, not recovery/retry of a failed simulation. |
| Invalid sample source, unknown Effect, or rejected journal | The coordinator returns a validation error before returning a successful monthly result. The caller's opening Ledger remains unchanged, with no partial transaction batch. |
| Duplicate output identity or conflicting state writes | Proposal merge rejects duplicate IDs and different values written to the same target before commit. These are merge-layer unit vectors; they do not claim a separate full-coordinator fixture. |
| Fatal error during accepted-input calculation | Stop the simulation and produce no successful result or snapshot for that month. No failed-run recovery, continuation, resource preservation, or replay is required. |

These rows record historical M4 test coverage. They do not define a production run
controller or recovery workflow. Current error and sampling boundaries are defined
by [ADR-19](adr/ADR-19-validation-only-domain-errors.md), [ADR-21](adr/ADR-21-m3-run-randomness-and-persistence.md),
[ADR-27](adr/ADR-27-current-month-runtime-and-database-owned-history.md), and
[ADR-29](adr/ADR-29-layered-calculation-and-application-boundaries.md). Correctable
input is rejected before calculation; a fatal calculation error returns no
successful result for that call and stops the simulation.

### Structural input errors

Reject negative annual interest rate, non-positive principal on an active loan, non-positive remaining term, invalid currency precision, a derived payment that rounds to zero, and terms effective other than at a month start. Zero annual rate and an absent/paid-off loan are valid. These vectors define calculation expectations; Ledger vectors are implemented with M1, and mortgage vectors with the mortgage milestone, not as placeholder tests in M0.
