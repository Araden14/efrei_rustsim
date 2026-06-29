use std::sync::Arc;
use tokio::sync::RwLock;
use crate::map::Pos;
use crate::world::SharedWorld;

pub async fn scout_loop(_world: Arc<RwLock<SharedWorld>>, _start: Pos) {
    // TODO
}

pub async fn collector_loop(_world: Arc<RwLock<SharedWorld>>, _start: Pos) {
    // TODO
}
