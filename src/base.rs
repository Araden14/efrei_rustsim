use crate::robot::RobotMessage;
use crate::world::SharedWorld;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, RwLock};

// ponytail: only wires the channel through to SharedWorld for now — aggregation
// rules (Communication System phase) land once robots actually send messages.
pub async fn run(world: Arc<RwLock<SharedWorld>>, mut rx: Receiver<RobotMessage>) {
    while let Some(msg) = rx.recv().await {
        let mut world = world.write().await;
        match msg {
            RobotMessage::Discovered { pos, cell } => {
                world.known_cells.insert(pos, cell);
            }
            RobotMessage::Collected { kind, amount } => match kind {
                crate::map::ResourceKind::Energy => world.energy_collected += amount,
                crate::map::ResourceKind::Crystal => world.crystal_collected += amount,
            },
        }
    }
}
