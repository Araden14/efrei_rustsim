use crate::map::{Cell, Pos, ResourceKind};
use std::collections::HashSet;
use tokio::sync::mpsc::Sender;

// Cardinal directions for movement: N, S, W, E
const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

// Moore neighborhood (8 surrounding cells) for discovery scanning
const NEIGHBORS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Clone, Debug)]
pub enum RobotMessage {
    Discovered { pos: Pos, cell: Cell },
    Collected { kind: ResourceKind, amount: u32 },
}

pub struct Robot {
    pub id: usize,
    pub kind: RobotKind,
    pub pos: Pos,
    pub known_cells: HashSet<Pos>,
    /// Channel to send discoveries and collection events to the base.
    pub tx: Sender<RobotMessage>,
    /// Resource the collector is currently carrying (None for scouts or idle collectors).
    pub carrying: Option<ResourceKind>,
}

impl Robot {
    pub fn new_scout(id: usize, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            kind: RobotKind::Scout,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None, // scouts never carry resources
        }
    }
}
