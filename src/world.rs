use crate::map::{Cell, Map, Pos, ResourceKind};
use crate::robot::RobotKind;
use std::collections::HashMap;

pub struct SharedWorld {
    pub map: Map,
    pub known_cells: HashMap<Pos, Cell>,
    pub robot_positions: HashMap<usize, Pos>,
    /// Stores the kind of each robot by id, used for rendering.
    pub robot_kinds: HashMap<usize, RobotKind>,
    pub energy_collected: u32,
    pub crystal_collected: u32,
    /// Maps each collector id to the resource position it has been dispatched to.
    /// A collector absent from this map is considered free and eligible for dispatch.
    pub collector_targets: HashMap<usize, Pos>,
}

impl SharedWorld {
    pub fn new(map: Map) -> Self {
        SharedWorld {
            map,
            known_cells: HashMap::new(),
            robot_positions: HashMap::new(),
            robot_kinds: HashMap::new(),
            energy_collected: 0,
            crystal_collected: 0,
            collector_targets: HashMap::new(),
        }
    }

    /// Assign a collector to a target resource position.
    pub fn assign_collector(&mut self, id: usize, target: Pos) {
        self.collector_targets.insert(id, target);
    }

    /// Mark a collector as free (no active target).
    pub fn free_collector(&mut self, id: usize) {
        self.collector_targets.remove(&id);
    }

    /// Increment the running total for the given resource kind.
    pub fn record_collection(&mut self, kind: ResourceKind, amount: u32) {
        match kind {
            ResourceKind::Energy => self.energy_collected += amount,
            ResourceKind::Crystal => self.crystal_collected += amount,
        }
    }
}
