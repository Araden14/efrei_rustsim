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
    let _tx = tx; // ponytail: kept alive so the base task doesn't exit; no robots send on it yet.
    tokio::spawn(base::run(world.clone(), rx));

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
