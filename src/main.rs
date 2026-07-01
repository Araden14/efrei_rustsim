mod config;
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

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // 1. Load configuration (config.toml if present, otherwise built-in defaults)
    let cfg = config::Config::load()?;

    // 2. Set up error reporting and enter the TUI
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<robot::RobotMessage>(256);

    // 3. Generate the world
    let seed: u32 = rand::random();
    let (viewport_width, viewport_height) = crossterm::terminal::size()?;
    let map_width = viewport_width.saturating_sub(2) as i32;
    let map_height = viewport_height.saturating_sub(3) as i32;
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width,
        map_height,
        seed,
        &cfg.map,
    ))));

    // 4. Spawn scouts and collectors at base_pos
    {
        let base_pos = world.read().await.map.base_pos();
        let scout_tick = Duration::from_millis(cfg.robots.scout_tick_ms);
        let collector_tick = Duration::from_millis(cfg.robots.collector_tick_ms);

        // Spawn scouts (ids 0..num_scouts)
        for id in 0..cfg.simulation.num_scouts {
            let scout = robot::Robot::new_scout(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(scout.id, scout.pos);
                w.robot_kinds.insert(scout.id, robot::RobotKind::Scout);
            }
            tokio::spawn(robot::run_scout(scout, world.clone(), scout_tick));
        }

        // Spawn collectors (ids num_scouts..num_scouts+num_collectors)
        for id in cfg.simulation.num_scouts
            ..(cfg.simulation.num_scouts + cfg.simulation.num_collectors)
        {
            let collector = robot::Robot::new_collector(id, base_pos, tx.clone());
            {
                let mut w = world.write().await;
                w.robot_positions.insert(collector.id, collector.pos);
                w.robot_kinds.insert(collector.id, robot::RobotKind::Collector);
            }
            tokio::spawn(robot::run_collector(collector, world.clone(), collector_tick));
        }
    }

    // 5. Spawn base task: reads messages from robots and updates world state
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
                    // The collector already removed itself from collector_targets before
                    // sending this message, so it will appear as free here.
                    let mut w = world_for_base.write().await;
                    dispatch_free_collectors(&mut w);
                }
            }
        }
    });

    // 6. Main loop: redraw at the configured interval, quit on keypress
    let ui_poll = Duration::from_millis(cfg.simulation.ui_poll_ms);
    let result = async {
        loop {
            {
                let world = world.read().await;
                terminal.draw(|frame| ui::render(frame, &world))?;
            }
            if crossterm::event::poll(ui_poll)?
                && crossterm::event::read()?.is_key_press()
            {
                return Ok(());
            }
        }
    }
    .await;

    // 7. Always restore the terminal
    ratatui::restore();
    result
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
            world.collector_targets.insert(collector_id, target);
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
