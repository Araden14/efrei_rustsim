mod map;
mod robot;
mod ui;
mod world;

use map::Map;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

const NUM_SCOUTS: usize = 10;
const NUM_COLLECTORS: usize = 5;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<robot::RobotMessage>(256);

    let seed: u32 = rand::random();
    let (viewport_width, viewport_height) = crossterm::terminal::size()?;
    let map_width = viewport_width.saturating_sub(2) as i32;
    let map_height = viewport_height.saturating_sub(3) as i32;
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width, map_height, seed,
    ))));

    {
        let base_pos = world.read().await.map.base_pos();

        for id in 0..NUM_SCOUTS {
            let scout = robot::Robot::new_scout(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(scout.id, scout.pos);
                w.robot_kinds.insert(scout.id, robot::RobotKind::Scout);
            }
            tokio::spawn(robot::run_scout(scout, world.clone()));
        }

        for id in NUM_SCOUTS..(NUM_SCOUTS + NUM_COLLECTORS) {
            let collector = robot::Robot::new_collector(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(collector.id, collector.pos);
                w.robot_kinds
                    .insert(collector.id, robot::RobotKind::Collector);
            }
            tokio::spawn(robot::run_collector(collector, world.clone()));
        }
    }

    let world_for_base = world.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                robot::RobotMessage::Discovered { pos, cell } => {
                    let mut w = world_for_base.write().await;
                    w.known_cells.insert(pos, cell);
                    // Any newly discovered resource may unblock idle collectors.
                    if matches!(cell, map::Cell::Resource(_, _)) {
                        dispatch_free_collectors(&mut w);
                    }
                }
                robot::RobotMessage::Collected { kind, amount } => {
                    let mut w = world_for_base.write().await;
                    match kind {
                        map::ResourceKind::Energy => w.energy_collected += amount,
                        map::ResourceKind::Crystal => w.crystal_collected += amount,
                    }
                }
                robot::RobotMessage::CollectorIdle(_id) => {
                    let mut w = world_for_base.write().await;
                    dispatch_free_collectors(&mut w);
                }
            }
        }
    });

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

    ratatui::restore();
    result
}

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
            world.collector_targets.insert(collector_id, target);
        }
    }
}

fn least_loaded_resource(world: &SharedWorld) -> Option<map::Pos> {
    world
        .known_cells
        .iter()
        .filter(|(_, cell)| matches!(cell, map::Cell::Resource(_, amount) if *amount > 0))
        .min_by_key(|(pos, _)| {
            world
                .collector_targets
                .values()
                .filter(|target| *target == pos)
                .count()
        })
        .map(|(pos, _)| *pos)
}
