use crate::map::{Cell, Pos};
use crate::robot::RobotType;
use crate::world::World;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render_ui(frame: &mut Frame, world: &World) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
        .split(frame.area());

    render_map(frame, world, chunks[0]);
    render_stats(frame, world, chunks[1]);
}

fn render_map(frame: &mut Frame, world: &World, area: Rect) {
    let mut map_text = String::new();

    for y in 0..world.map.height {
        for x in 0..world.map.width {
            let pos = Pos { x, y };
            let ch = if let Some(robot) = world.robots.iter().find(|robot| robot.pos == pos) {
                match robot.robot_type {
                    RobotType::Scout => 'x',
                    RobotType::Collector => 'o',
                }
            } else if world.base.pos == pos {
                '#'
            } else {
                match world.map.grid[y][x] {
                    Cell::Empty => '.',
                    Cell::Obstacle => 'O',
                    Cell::Energy(_) => 'E',
                    Cell::Crystal(_) => 'C',
                }
            };

            map_text.push(ch);
        }
        map_text.push('\n');
    }

    frame.render_widget(Paragraph::new(map_text), area);
}

fn render_stats(frame: &mut Frame, world: &World, area: Rect) {
    let carried_energy: u32 = world.robots.iter().map(|robot| robot.carrying.0).sum();
    let carried_crystals: u32 = world.robots.iter().map(|robot| robot.carrying.1).sum();
    let stats = format!(
        "Energy: {} (+{}) | Crystals: {} (+{}) | Discoveries: {} | Tick: {} | q/esc to quit",
        world.base.total_energy,
        carried_energy,
        world.base.total_crystals,
        carried_crystals,
        world.base.discovered_resources.len(),
        world.tick
    );

    frame.render_widget(
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL)),
        area,
    );
}
