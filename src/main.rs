mod map;
mod robots;
mod ui;
mod world;

use map::Map;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use world::SharedWorld;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (scout_count, collector_count) = setup()?;

    let mut terminal = ratatui::init();
    let size = terminal.size()?;
    let map_width = size.width.saturating_sub(2).max(10) as i32;
    let map_height = size.height.saturating_sub(3).max(10) as i32;

    let seed: u32 = rand::random();
    let world = Arc::new(RwLock::new(SharedWorld::new(Map::generate(
        map_width,
        map_height,
        seed,
    ))));

    let base_pos = world.read().await.base_pos;

    for _ in 0..scout_count {
        tokio::spawn(robots::scout_loop(world.clone(), base_pos));
    }
    for _ in 0..collector_count {
        tokio::spawn(robots::collector_loop(world.clone(), base_pos));
    }

    let result = run(&mut terminal, world).await;
    ratatui::restore();
    result
}

fn setup() -> color_eyre::Result<(usize, usize)> {
    let scouts = prompt_count("How many scouts?")?;
    let collectors = prompt_count("How many collectors?")?;
    Ok((scouts, collectors))
}

fn prompt_count(label: &str) -> color_eyre::Result<usize> {
    use std::io::{self, Write};
    loop {
        print!("{} > ", label);
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        match line.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= 10 => return Ok(n),
            _ => println!("Enter a number between 1 and 10."),
        }
    }
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
