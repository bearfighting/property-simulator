//! In-memory lifecycle and identity for the one simulation session exposed by
//! this desktop workflow.

use property_simulation_lifecycle::{SimulationState, WorkUnit, new};
use std::{cell::Cell, cell::RefCell, rc::Rc};

#[derive(Clone)]
pub(super) struct SimulationSession {
    state: Rc<Cell<SimulationState>>,
    simulation_id: Rc<RefCell<Option<String>>>,
}

impl SimulationSession {
    pub(super) fn new() -> Self {
        Self {
            state: Rc::new(Cell::new(new())),
            simulation_id: Rc::new(RefCell::new(None)),
        }
    }

    pub(super) fn state(&self) -> SimulationState {
        self.state.get()
    }

    pub(super) fn allowed_work(&self) -> WorkUnit {
        self.state().allowed_work()
    }

    pub(super) fn simulation_id(&self) -> Option<String> {
        self.simulation_id.borrow().clone()
    }

    pub(super) fn initialize(&self, simulation_id: String) -> Option<SimulationState> {
        if self.allowed_work() != WorkUnit::InitialSettings {
            return None;
        }

        let next = self.state().transition()?;
        *self.simulation_id.borrow_mut() = Some(simulation_id);
        self.state.set(next);
        Some(next)
    }

    pub(super) fn stop(&self) -> Option<SimulationState> {
        if self.allowed_work() != WorkUnit::MonthSimulation {
            return None;
        }

        let next = self.state().transition()?;
        self.state.set(next);
        Some(next)
    }

    pub(super) fn finish_month(&self, should_stop: bool) -> SimulationState {
        let current = self.state();
        if should_stop && current.allowed_work() == WorkUnit::MonthSimulation {
            if let Some(next) = current.transition() {
                self.state.set(next);
            }
        }
        self.state()
    }

    /// Abandon the current run and start a new in-memory lifecycle.
    /// Previously persisted simulation data is left untouched.
    pub(super) fn reset(&self) -> Option<SimulationState> {
        if self.allowed_work() == WorkUnit::InitialSettings {
            return None;
        }

        *self.simulation_id.borrow_mut() = None;
        let initial = new();
        self.state.set(initial);
        Some(initial)
    }
}
