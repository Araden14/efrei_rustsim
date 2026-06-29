use crate::map::{Pos, ResourceKind};
use crate::robot::RobotMessage;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

pub struct Base {
    pub pos: Pos,
    pub total_energy: u32,
    pub total_crystals: u32,
    pub discovered_resources: HashMap<Pos, ResourceKind>,
    rx: Receiver<RobotMessage>,
}

impl Base {
    pub fn new(pos: Pos, rx: Receiver<RobotMessage>) -> Self {
        Base {
            pos,
            total_energy: 0,
            total_crystals: 0,
            discovered_resources: HashMap::new(),
            rx,
        }
    }

    pub fn drain_messages(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                RobotMessage::Discovered {
                    pos, kind, amount, ..
                } => {
                    if amount > 0 {
                        self.discovered_resources.insert(pos, kind);
                    } else {
                        self.discovered_resources.remove(&pos);
                    }
                }
                RobotMessage::Collected { kind, amount, .. } => match kind {
                    ResourceKind::Energy => self.total_energy += amount,
                    ResourceKind::Crystal => self.total_crystals += amount,
                },
            }
        }
    }

    pub fn forget_resource(&mut self, pos: Pos) {
        self.discovered_resources.remove(&pos);
    }
}
