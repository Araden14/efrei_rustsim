mod base;
mod map;
mod robot;
mod ui;
mod world;

use ratatui::crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode},
};
use std::io;
use std::time::Duration;
use world::World;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?
        .with_cursor_visible(true);

    let result = run_app(&mut terminal).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    let mut world = World::new(100, 30);
    let mut should_quit = false;

    while !should_quit {
        // Render
        terminal.draw(|f| ui::render_ui(f, &world))?;

        // Update
        world.update();

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    should_quit = true;
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
