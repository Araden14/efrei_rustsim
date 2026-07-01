mod map;
mod robot;
mod ui;
mod world;

use map::Map;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // 1. set up error reporting and enter the TUI (raw mode + alternate screen)
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<robot::RobotMessage>();

    // 2. generate the world, sized to fit the map area (terminal minus the
    //    bordered block's 2 rows/cols and the 1-row status bar) so the base
    //    actually lands in the center of what's drawn on screen
    let seed: u32 = rand::random();
    let (viewport_width, viewport_height) = crossterm::terminal::size()?;
    let map_width = viewport_width.saturating_sub(2) as i32;
    let map_height = viewport_height.saturating_sub(3) as i32;
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width,
        map_height,
        seed,
    ))));
    
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

    // 3. main loop: redraw, then wait briefly for a keypress to quit
    let result = async {
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
    .await;

    // 4. always restore the terminal, even if the loop returned an error
    ratatui::restore();
    result
}

