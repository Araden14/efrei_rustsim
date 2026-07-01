mod map;
mod robot;
mod ui;
mod world;

use map::Map;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

// Number of scouts and collectors to spawn
const NUM_SCOUTS: usize = ;
const NUM_COLLECTORS: usize = 2;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // 1. Set up error reporting and enter the TUI
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<robot::RobotMessage>(256);

    // 2. Generate the world
    let seed: u32 = rand::random();
    let (viewport_width, viewport_height) = crossterm::terminal::size()?;
    let map_width = viewport_width.saturating_sub(2) as i32;
    let map_height = viewport_height.saturating_sub(3) as i32;
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
            let scout = robot::Robot::new_scout(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(scout.id, scout.pos);
                w.robot_kinds.insert(scout.id, robot::RobotKind::Scout);
            }
            tokio::spawn(robot::run_scout(scout, world.clone()));
        }

        // Spawn NUM_COLLECTORS collectors (ids NUM_SCOUTS..NUM_SCOUTS+NUM_COLLECTORS)
        for id in NUM_SCOUTS..(NUM_SCOUTS + NUM_COLLECTORS) {
            let collector = robot::Robot::new_collector(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(collector.id, collector.pos);
                w.robot_kinds.insert(collector.id, robot::RobotKind::Collector);
            }
            tokio::spawn(robot::run_collector(collector, world.clone()));
        }
    }

    // 4. Spawn base task: reads messages from robots and updates world state
    let world_for_base = world.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                robot::RobotMessage::Discovered { pos, cell } => {
                    let mut w = world_for_base.write().await;
                    w.known_cells.insert(pos, cell);

                    // If this is a resource that no collector is already heading toward,
                    // dispatch the first free collector to it.
                    if matches!(cell, map::Cell::Resource(_, _)) {
                        let already_targeted = w.collector_targets.values().any(|&t| t == pos);
                        if !already_targeted {
                            let free_id = w
                                .robot_kinds
                                .iter()
                                .filter(|(_, k)| **k == robot::RobotKind::Collector)
                                .map(|(id, _)| *id)
                                .find(|id| !w.collector_targets.contains_key(id));
                            if let Some(collector_id) = free_id {
                                w.collector_targets.insert(collector_id, pos);
                            }
                        }
                    }
                }
                robot::RobotMessage::Collected { kind, amount } => {
                    let mut w = world_for_base.write().await;
                    match kind {
                        map::ResourceKind::Energy => w.energy_collected += amount,
                        map::ResourceKind::Crystal => w.crystal_collected += amount,
                    }
                }
                robot::RobotMessage::CollectorIdle(id) => {
                    let mut w = world_for_base.write().await;
                    // Build the set of resource positions already claimed by other collectors.
                    let already_targeted: std::collections::HashSet<map::Pos> =
                        w.collector_targets.values().copied().collect();
                    // Find any known resource that is not yet assigned to someone.
                    let next_target = w
                        .known_cells
                        .iter()
                        .filter_map(|(pos, cell)| match cell {
                            map::Cell::Resource(_, amount) if *amount > 0 => Some(*pos),
                            _ => None,
                        })
                        .find(|pos| !already_targeted.contains(pos));
                    if let Some(resource_pos) = next_target {
                        w.collector_targets.insert(id, resource_pos);
                    }
                }
            }
        }
    });

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