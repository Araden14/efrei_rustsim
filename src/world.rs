use crate::map::{Map, Pos, ResourceKind};

pub struct Scout {
    pub pos: Pos,
}

pub struct Collector {
    pub pos: Pos,
    pub carrying: Option<ResourceKind>,
    pub target: Option<Pos>,
}

pub struct SharedWorld {
    pub map: Map,
    pub base_pos: Pos,
    pub known_resources: Vec<(Pos, ResourceKind)>,
    pub energy_collected: u32,
    pub crystal_collected: u32,
}

impl SharedWorld {
    pub fn new(map: Map) -> Self {
        let base_pos = map.base_pos();
        SharedWorld {
            map,
            base_pos,
            known_resources: Vec::new(),
            energy_collected: 0,
            crystal_collected: 0,
        }
    }
}
