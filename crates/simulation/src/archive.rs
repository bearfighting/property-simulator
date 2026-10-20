use std::collections::BTreeMap;

use property_simulation_domain::{
    ActivityInstanceId, BranchId, RunId, SnapshotId, TrialId, YearMonth,
};

use crate::{
    ActivityInstance, CashFlowContributionTotals, FrozenInput, LoanContinuation, MonthSnapshot,
    OpeningState, RunConfig, SimulationError, SimulationRun,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCashFlowSummary {
    pub by_month: BTreeMap<YearMonth, CashFlowContributionTotals>,
    pub by_activity: BTreeMap<ActivityInstanceId, CashFlowContributionTotals>,
    pub cumulative: CashFlowContributionTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedMonth {
    pub id: SnapshotId,
    pub run_id: RunId,
    pub branch_id: BranchId,
    pub parent_snapshot: Option<SnapshotId>,
    pub snapshot: MonthSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRecord {
    pub id: BranchId,
    pub run_id: RunId,
    pub trial_id: TrialId,
    pub start_month: YearMonth,
    pub parent_snapshot: Option<SnapshotId>,
    pub fork_month: Option<YearMonth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSettings {
    pub config: RunConfig,
    pub opening_state: OpeningState,
    pub frozen_input: FrozenInput,
    pub activities: Vec<ActivityInstance>,
    pub loan_continuations: Vec<LoanContinuation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    DuplicateRun,
    DuplicateBranch,
    DuplicateTrial,
    DuplicateSnapshot,
    MissingParentSnapshot,
    MissingBranch,
    InvalidReference,
    NonContiguousMonth,
    NoSuccessorMonth,
    InvalidRunBoundary,
}

#[derive(Debug, Default)]
pub struct SimulationArchive {
    runs: BTreeMap<RunId, ()>,
    branches: BTreeMap<BranchId, BranchRecord>,
    snapshots: BTreeMap<SnapshotId, SavedMonth>,
    latest_settings: Option<UserSettings>,
}

impl SimulationArchive {
    pub fn register_branch(&mut self, record: BranchRecord) -> Result<(), ArchiveError> {
        if self.runs.contains_key(&record.run_id) {
            return Err(ArchiveError::DuplicateRun);
        }
        if self.branches.contains_key(&record.id) {
            return Err(ArchiveError::DuplicateBranch);
        }
        if self
            .branches
            .values()
            .any(|branch| branch.trial_id == record.trial_id)
        {
            return Err(ArchiveError::DuplicateTrial);
        }
        if let Some(parent_id) = record.parent_snapshot {
            let parent = self
                .snapshots
                .get(&parent_id)
                .ok_or(ArchiveError::MissingParentSnapshot)?;
            let next_month = parent
                .snapshot
                .result()
                .period()
                .checked_add_months(1)
                .map_err(|_| ArchiveError::NoSuccessorMonth)?;
            if record.fork_month != Some(parent.snapshot.result().period())
                || record.start_month != next_month
            {
                return Err(ArchiveError::InvalidReference);
            }
        } else if record.fork_month.is_some() {
            return Err(ArchiveError::InvalidReference);
        }
        self.runs.insert(record.run_id, ());
        self.branches.insert(record.id, record);
        Ok(())
    }

    pub fn save_month(&mut self, saved: SavedMonth) -> Result<(), ArchiveError> {
        if self.snapshots.contains_key(&saved.id) {
            return Err(ArchiveError::DuplicateSnapshot);
        }
        let branch = self
            .branches
            .get(&saved.branch_id)
            .ok_or(ArchiveError::MissingBranch)?;
        if branch.run_id != saved.run_id
            || saved.snapshot.result().branch_id() != saved.branch_id
            || saved.snapshot.result().trial_id() != branch.trial_id
        {
            return Err(ArchiveError::InvalidReference);
        }
        let month = saved.snapshot.result().period();
        let previous = self
            .snapshots
            .values()
            .filter(|item| item.branch_id == saved.branch_id)
            .max_by_key(|item| item.snapshot.result().period())
            .map(|item| item.id);
        let expected_parent = previous.or(branch.parent_snapshot);
        if saved.parent_snapshot != expected_parent {
            return Err(ArchiveError::InvalidReference);
        }
        let expected_month = if let Some(parent_id) = expected_parent {
            let parent = self
                .snapshots
                .get(&parent_id)
                .ok_or(ArchiveError::MissingParentSnapshot)?;
            parent
                .snapshot
                .result()
                .period()
                .checked_add_months(1)
                .map_err(|_| ArchiveError::NoSuccessorMonth)?
        } else if let Some(parent_id) = branch.parent_snapshot {
            self.snapshots
                .get(&parent_id)
                .ok_or(ArchiveError::MissingParentSnapshot)?
                .snapshot
                .result()
                .period()
                .checked_add_months(1)
                .map_err(|_| ArchiveError::NoSuccessorMonth)?
        } else {
            branch.start_month
        };
        if month != expected_month {
            return Err(ArchiveError::NonContiguousMonth);
        }
        self.snapshots.insert(saved.id, saved);
        Ok(())
    }

    pub fn snapshot(&self, id: SnapshotId) -> Option<&SavedMonth> {
        self.snapshots.get(&id)
    }
    pub fn branch(&self, id: BranchId) -> Option<&BranchRecord> {
        self.branches.get(&id)
    }
    pub fn update_latest_settings(&mut self, settings: UserSettings) {
        self.latest_settings = Some(settings);
    }
    pub fn latest_settings(&self) -> Option<&UserSettings> {
        self.latest_settings.as_ref()
    }

    /// Derive this run's saved-period totals; parent branch history is excluded.
    pub fn cash_flow_summary(&self, run_id: RunId) -> Result<RunCashFlowSummary, SimulationError> {
        let currency = self
            .snapshots
            .values()
            .find(|saved| saved.run_id == run_id)
            .map(|saved| saved.snapshot.result().closing_state().cash.currency())
            .ok_or(SimulationError::InvalidInput(
                "run has no saved months to summarize",
            ))?;
        let mut month_amounts = BTreeMap::<YearMonth, Vec<_>>::new();
        let mut activity_amounts = BTreeMap::<ActivityInstanceId, Vec<_>>::new();
        let mut cumulative_amounts = Vec::new();
        for saved in self
            .snapshots
            .values()
            .filter(|saved| saved.run_id == run_id)
        {
            let result = saved.snapshot.result();
            month_amounts.entry(result.period()).or_default();
            for contribution in result.cash_flow_contributions() {
                month_amounts
                    .entry(result.period())
                    .or_default()
                    .push((contribution.category, contribution.amount));
                activity_amounts
                    .entry(contribution.instance_id)
                    .or_default()
                    .push((contribution.category, contribution.amount));
                cumulative_amounts.push((contribution.category, contribution.amount));
            }
            for contribution in result.input_cash_flow_contributions() {
                month_amounts
                    .entry(result.period())
                    .or_default()
                    .push((contribution.category, contribution.amount));
                cumulative_amounts.push((contribution.category, contribution.amount));
            }
        }
        let by_month = month_amounts
            .into_iter()
            .map(|(month, amounts)| {
                Ok((month, CashFlowContributionTotals::sum(amounts, currency)?))
            })
            .collect::<Result<_, SimulationError>>()?;
        let by_activity = activity_amounts
            .into_iter()
            .map(|(activity, amounts)| {
                Ok((
                    activity,
                    CashFlowContributionTotals::sum(amounts, currency)?,
                ))
            })
            .collect::<Result<_, SimulationError>>()?;
        let cumulative = CashFlowContributionTotals::sum(cumulative_amounts, currency)?;
        Ok(RunCashFlowSummary {
            by_month,
            by_activity,
            cumulative,
        })
    }

    pub fn create_branch(
        &mut self,
        parent_id: SnapshotId,
        run_id: RunId,
        branch_id: BranchId,
        trial_id: TrialId,
        end: Option<YearMonth>,
        stop_conditions: Vec<crate::StopCondition>,
    ) -> Result<SimulationRun, ArchiveError> {
        if self.runs.contains_key(&run_id) {
            return Err(ArchiveError::DuplicateRun);
        }
        if self.branches.contains_key(&branch_id) {
            return Err(ArchiveError::DuplicateBranch);
        }
        let parent = self
            .snapshots
            .get(&parent_id)
            .ok_or(ArchiveError::MissingParentSnapshot)?;
        let next = parent
            .snapshot
            .result()
            .period()
            .checked_add_months(1)
            .map_err(|_| ArchiveError::NoSuccessorMonth)?;
        if end.is_some_and(|end| end < next) {
            return Err(ArchiveError::InvalidRunBoundary);
        }
        let result = parent.snapshot.result();
        let mut input = *result.frozen_input();
        input.period = next;
        let mut run = SimulationRun::new_with_activities_and_loans(
            RunConfig {
                start: next,
                end,
                stop_conditions,
            },
            result.closing_state().clone(),
            input,
            parent.snapshot.ledger().clone(),
            result.activities().to_vec(),
            result.loan_continuations().to_vec(),
        )
        .map_err(|_| ArchiveError::InvalidRunBoundary)?;
        run.set_execution_identity(trial_id, branch_id);
        self.register_branch(BranchRecord {
            id: branch_id,
            run_id,
            trial_id,
            start_month: next,
            parent_snapshot: Some(parent_id),
            fork_month: Some(result.period()),
        })?;
        Ok(run)
    }
}

pub fn saved_month(
    id: SnapshotId,
    run_id: RunId,
    snapshot: MonthSnapshot,
    parent_snapshot: Option<SnapshotId>,
) -> SavedMonth {
    SavedMonth {
        id,
        run_id,
        branch_id: snapshot.result().branch_id(),
        parent_snapshot,
        snapshot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_registration_rejects_missing_parent_and_duplicate_identity() {
        let mut archive = SimulationArchive::default();
        let record = BranchRecord {
            id: BranchId::from_bytes([1; 16]),
            run_id: RunId::from_bytes([2; 16]),
            trial_id: TrialId::from_bytes([3; 16]),
            start_month: YearMonth::new(2025, 2).unwrap(),
            parent_snapshot: Some(SnapshotId::from_bytes([4; 16])),
            fork_month: Some(YearMonth::new(2025, 1).unwrap()),
        };
        assert_eq!(
            archive.register_branch(record.clone()),
            Err(ArchiveError::MissingParentSnapshot)
        );
        let root = BranchRecord {
            start_month: YearMonth::new(2025, 1).unwrap(),
            parent_snapshot: None,
            fork_month: None,
            ..record
        };
        archive.register_branch(root.clone()).unwrap();
        assert_eq!(
            archive.register_branch(root),
            Err(ArchiveError::DuplicateRun)
        );
    }

    #[test]
    fn branch_registration_rejects_trial_id_reuse() {
        let mut archive = SimulationArchive::default();
        let trial_id = TrialId::from_bytes([3; 16]);
        archive
            .register_branch(BranchRecord {
                id: BranchId::from_bytes([1; 16]),
                run_id: RunId::from_bytes([2; 16]),
                trial_id,
                start_month: YearMonth::new(2025, 1).unwrap(),
                parent_snapshot: None,
                fork_month: None,
            })
            .unwrap();
        assert_eq!(
            archive.register_branch(BranchRecord {
                id: BranchId::from_bytes([4; 16]),
                run_id: RunId::from_bytes([5; 16]),
                trial_id,
                start_month: YearMonth::new(2025, 1).unwrap(),
                parent_snapshot: None,
                fork_month: None,
            }),
            Err(ArchiveError::DuplicateTrial)
        );
    }
}
