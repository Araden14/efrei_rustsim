use crate::map::{Cell, Map, Pos};
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
        }
    }
}