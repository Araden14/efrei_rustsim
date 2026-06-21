use crate::map::Map;

pub struct SharedWorld {
    pub map: Map,
    pub energy_collected: u32,
    pub crystal_collected: u32,
}

impl SharedWorld {
    pub fn new(map: Map) -> Self {
        SharedWorld { map, energy_collected: 0, crystal_collected: 0 }
    }
}
