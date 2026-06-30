mod base;
mod map;
mod robot;
mod ui;
mod world;

use map::Map;
use robot::RobotMessage;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let world = Arc::new(RwLock::new(SharedWorld::new(Map::empty(60, 30))));

    let (tx, rx) = tokio::sync::mpsc::channel::<RobotMessage>(100);
    tokio::spawn(base::run(world.clone(), rx));

    // TODO (step 5 — remove before Phase 2 merge): temporary single scout for local testing.
    {
        let base_pos = world.read().await.map.base_pos();
        let scout = robot::Robot::new_scout(0, base_pos, tx.clone());
        {
            let mut w = world.write().await;
            w.robot_positions.insert(scout.id, scout.pos);
            w.robot_kinds.insert(scout.id, robot::RobotKind::Scout);
        }
        tokio::spawn(robot::run_scout(scout, world.clone()));
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, world).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    world: Arc<RwLock<SharedWorld>>,
) -> color_eyre::Result<()> {
    loop {
        {
            let world = world.read().await;
            terminal.draw(|frame| ui::render(frame, &world))?;
        }
        if crossterm::event::poll(Duration::from_millis(100))? && crossterm::event::read()?.is_key_press() {
            return Ok(());
        }
    }
}
