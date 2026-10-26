use property_simulation_lifecycle::{SimulationState, WorkUnit, new};

#[test]
fn each_state_has_one_work_unit_and_a_linear_successor() {
    let initial = new();
    assert_eq!(initial, SimulationState::Initial);
    assert_eq!(initial.allowed_work(), WorkUnit::InitialSettings);

    let running = initial.transition().unwrap();
    assert_eq!(running, SimulationState::Running);
    assert_eq!(running.allowed_work(), WorkUnit::MonthSimulation);

    let reporting = running.transition().unwrap();
    assert_eq!(reporting, SimulationState::Reporting);
    assert_eq!(reporting.allowed_work(), WorkUnit::Reporting);
    assert_eq!(reporting.transition(), None);
}

#[test]
fn running_state_can_be_reused_for_multiple_months() {
    let running = SimulationState::Initial.transition().unwrap();
    assert_eq!(running.allowed_work(), WorkUnit::MonthSimulation);
    assert_eq!(running.allowed_work(), WorkUnit::MonthSimulation);
    assert_eq!(running.transition(), Some(SimulationState::Reporting));
}
