use crate::map::{Cell, Pos, ResourceKind};
use std::collections::HashSet;
use tokio::sync::mpsc::Sender;

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
