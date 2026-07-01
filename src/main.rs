mod map;
mod robot;
mod ui;
mod world;

use map::Map;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

// Number of scouts and collectors to spawn
const NUM_SCOUTS: usize = 10;
const NUM_COLLECTORS: usize = 5;

/// Left + right border of the map widget (1 char each).
const MAP_BORDER_X: u16 = 2;
/// Top + bottom border of the map widget (1 char each) + 1 char for the status bar.
const MAP_BORDER_Y: u16 = 3;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Enter the TUI
    let mut terminal = ratatui::init();
    let (tx, rx) = tokio::sync::mpsc::channel::<robot::RobotMessage>(256);

    // 2. Generate the world
    let seed: u32 = rand::random();
    let (viewport_width, viewport_height) = crossterm::terminal::size()?;
    let map_width  = viewport_width.saturating_sub(MAP_BORDER_X) as i32;
    let map_height = viewport_height.saturating_sub(MAP_BORDER_Y) as i32;
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width,
        map_height,
        seed,
    ))));

    // 3. Spawn scouts and collectors at base_pos
    {
        let base_pos = world.read().await.map.base_pos();

        // Spawn NUM_SCOUTS scouts (ids 0..NUM_SCOUTS)
        for id in 0..NUM_SCOUTS {
            let scout = robot::Robot::new(id, robot::RobotKind::Scout, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(scout.id, scout.pos);
                w.robot_kinds.insert(scout.id, robot::RobotKind::Scout);
            }
            tokio::spawn(robot::run_scout(scout, world.clone()));
        }

        // Spawn NUM_COLLECTORS collectors (ids NUM_SCOUTS..NUM_SCOUTS+NUM_COLLECTORS)
        for id in NUM_SCOUTS..(NUM_SCOUTS + NUM_COLLECTORS) {
            let collector = robot::Robot::new(id, robot::RobotKind::Collector, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(collector.id, collector.pos);
                w.robot_kinds.insert(collector.id, robot::RobotKind::Collector);
            }
            tokio::spawn(robot::run_collector(collector, world.clone()));
        }
    }

    // 4. Spawn base task: reads messages from robots and updates world state
    tokio::spawn(run_base(rx, world.clone()));

    // 5. Main loop: redraw every 50ms, quit on keypress
    let result = async {
        loop {
            {
                let world = world.read().await;
                terminal.draw(|frame| ui::render(frame, &world))?;
            }
            if crossterm::event::poll(Duration::from_millis(50))?
                && crossterm::event::read()?.is_key_press()
            {
                return Ok(());
            }
        }
    }
    .await;

    // 6. Always restore the terminal
    ratatui::restore();
    result
}

/// Reads robot messages and updates world state on behalf of the base.
async fn run_base(
    mut rx: tokio::sync::mpsc::Receiver<robot::RobotMessage>,
    world: Arc<RwLock<SharedWorld>>,
) {
    while let Some(msg) = rx.recv().await {
        match msg {
            robot::RobotMessage::Discovered { pos, cell } => {
                let mut w = world.write().await;
                w.known_cells.insert(pos, cell);
                // Any newly discovered resource may unblock idle collectors.
                if matches!(cell, map::Cell::Resource(_, _)) {
                    dispatch_free_collectors(&mut w);
                }
            }
            robot::RobotMessage::Collected { kind, amount } => {
                let mut w = world.write().await;
                w.record_collection(kind, amount);
            }
            robot::RobotMessage::CollectorIdle => {
                // The collector already removed itself from collector_targets before
                // sending this message, so it will appear as free here.
                let mut w = world.write().await;
                dispatch_free_collectors(&mut w);
            }
        }
    }
}

/// Assign every unassigned collector to a resource, preferring the resource
/// with the fewest collectors already heading toward it.
///
/// Because assignment counts are updated inside the loop, consecutive calls for
/// multiple free collectors naturally spread them across available resources
/// before any single resource gets a second assignment.
fn dispatch_free_collectors(world: &mut SharedWorld) {
    let free_collectors: Vec<usize> = world
        .robot_kinds
        .iter()
        .filter(|(_, k)| **k == robot::RobotKind::Collector)
        .map(|(id, _)| *id)
        .filter(|id| !world.collector_targets.contains_key(id))
        .collect();

    for collector_id in free_collectors {
        if let Some(target) = least_loaded_resource(world) {
            world.assign_collector(collector_id, target);
        }
    }
}

/// Return the known resource position that currently has the fewest collectors
/// assigned to it, or `None` if no resources are known yet.
fn least_loaded_resource(world: &SharedWorld) -> Option<map::Pos> {
    // Build a count map: resource_pos -> number of collectors already assigned.
    let mut counts: HashMap<map::Pos, usize> = world
        .known_cells
        .iter()
        .filter_map(|(pos, cell)| match cell {
            map::Cell::Resource(_, amount) if *amount > 0 => Some((*pos, 0usize)),
            _ => None,
        })
        .collect();

    for &target in world.collector_targets.values() {
        if let Some(c) = counts.get_mut(&target) {
            *c += 1;
        }
    }

    counts.into_iter().min_by_key(|(_, c)| *c).map(|(pos, _)| pos)
}
