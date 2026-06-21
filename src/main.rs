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

    let mut terminal = ratatui::init();
    let size = terminal.size()?;
    // map_area is the frame minus the 1-row status line; the bordered block then
    // eats 1 char on every side, so subtract 3 rows / 2 cols to match ui::render's layout exactly.
    let map_width = size.width.saturating_sub(2).max(10) as i32;
    let map_height = size.height.saturating_sub(3).max(10) as i32;

    let seed: u32 = rand::random();
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width,
        map_height,
        seed,
    ))));

    let (tx, rx) = tokio::sync::mpsc::channel::<RobotMessage>(100);
    let _tx = tx; // ponytail: kept alive so the base task doesn't exit; no robots send on it yet.
    tokio::spawn(base::run(world.clone(), rx));

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
