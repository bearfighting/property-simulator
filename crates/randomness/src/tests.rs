use crate::{RandomCategory, RandomStreams, RootKey, RunKeyFactory, SemanticIdentity};
use property_simulation_domain::{ActivityInstanceId, SliceId, TrialId, YearMonth};
use property_simulation_models::RandomSource;

fn identity() -> SemanticIdentity {
    SemanticIdentity {
        trial_id: TrialId::from_bytes([1; 16]),
        category: RandomCategory::Property,
        slice_id: SliceId::from_bytes([2; 16]),
        activity_id: ActivityInstanceId::from_bytes([3; 16]),
        period: YearMonth::new(2026, 1).unwrap(),
        purpose: "repair-cost".into(),
    }
}

fn draws(streams: &RandomStreams, id: &SemanticIdentity) -> [u64; 3] {
    let mut stream = streams.stream(id).unwrap();
    std::array::from_fn(|_| stream.next_u64().unwrap())
}

#[test]
fn entropy_success_and_failure_are_explicit() {
    assert!(
        RootKey::from_entropy(|out| {
            out.fill(9);
            Ok::<_, ()>(())
        })
        .is_ok()
    );
    assert!(RootKey::from_entropy(|_| Err(())).is_err());
}

#[test]
fn run_keys_are_derived_from_trial_identity() {
    let root = [8; 32];
    let a = RunKeyFactory::new(RootKey::from_seed(root)).run_streams(TrialId::from_bytes([1; 16]));
    let b = RunKeyFactory::new(RootKey::from_seed(root)).run_streams(TrialId::from_bytes([1; 16]));
    let id = identity();
    assert_eq!(draws(&a, &id), draws(&b, &id));

    let different_trial =
        RunKeyFactory::new(RootKey::from_seed(root)).run_streams(TrialId::from_bytes([2; 16]));
    let mut different_trial_identity = id.clone();
    different_trial_identity.trial_id = TrialId::from_bytes([2; 16]);
    assert_ne!(
        draws(&a, &id),
        draws(&different_trial, &different_trial_identity)
    );
}

#[test]
fn semantic_fields_and_independent_consumption_are_isolated() {
    let streams =
        RunKeyFactory::new(RootKey::from_seed([7; 32])).run_streams(TrialId::from_bytes([1; 16]));
    let base = identity();
    let expected = draws(&streams, &base);
    let mut unrelated = streams
        .stream(&SemanticIdentity {
            purpose: "other".into(),
            ..base.clone()
        })
        .unwrap();
    for _ in 0..100 {
        let _ = unrelated.next_u64().unwrap();
    }
    assert_eq!(draws(&streams, &base), expected);
    let mut changed = base.clone();
    changed.period = changed.period.checked_add_months(1).unwrap();
    assert_ne!(draws(&streams, &changed), expected);
    changed = base.clone();
    changed.activity_id = ActivityInstanceId::from_bytes([5; 16]);
    assert_ne!(draws(&streams, &changed), expected);
    changed = base.clone();
    changed.slice_id = SliceId::from_bytes([6; 16]);
    assert_ne!(draws(&streams, &changed), expected);
    changed = base.clone();
    changed.category = RandomCategory::Environment;
    assert_ne!(draws(&streams, &changed), expected);
    changed = base.clone();
    changed.period = YearMonth::new(2027, 1).unwrap();
    assert_ne!(draws(&streams, &changed), expected);
    changed = base.clone();
    changed.purpose = "repair-occurrence".into();
    assert_ne!(draws(&streams, &changed), expected);
}

#[test]
fn semantic_stream_rejects_a_different_trial_identity() {
    let streams =
        RunKeyFactory::new(RootKey::from_seed([4; 32])).run_streams(TrialId::from_bytes([1; 16]));
    let mut foreign = identity();
    foreign.trial_id = TrialId::from_bytes([2; 16]);
    assert!(matches!(
        streams.stream(&foreign),
        Err(crate::RandomnessError::TrialIdentityMismatch)
    ));
}
