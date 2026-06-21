use crate::map::{Cell, Map, Pos};
use std::collections::HashMap;

pub struct SharedWorld {
    pub map: Map,
    pub known_cells: HashMap<Pos, Cell>,
    pub robot_positions: HashMap<usize, Pos>,
    pub energy_collected: u32,
    pub crystal_collected: u32,
}

impl SharedWorld {
    pub fn new(map: Map) -> Self {
        SharedWorld {
            map,
            known_cells: HashMap::new(),
            robot_positions: HashMap::new(),
            energy_collected: 0,
            crystal_collected: 0,
        }
    }
}
