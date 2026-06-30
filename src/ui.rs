use crate::map::Cell;
use crate::robot::RobotKind;
use crate::world::SharedWorld;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

fn cell_glyph(cell: Cell) -> (char, Color) {
    match cell {
        Cell::Empty => (' ', Color::Reset),
        Cell::Obstacle => ('O', Color::LightCyan),
        Cell::Resource(crate::map::ResourceKind::Energy, _) => ('E', Color::Green),
        Cell::Resource(crate::map::ResourceKind::Crystal, _) => ('C', Color::LightMagenta),
        Cell::Base => ('#', Color::LightGreen),
    }
}

fn robot_glyph(kind: RobotKind) -> (char, Color) {
    match kind {
        RobotKind::Scout => ('x', Color::Red),
        RobotKind::Collector => ('o', Color::Magenta),
    }
}

pub fn render(frame: &mut Frame, world: &SharedWorld) {
    let [map_area, status_area] = Layout::new(
        Direction::Vertical,
        [Constraint::Min(0), Constraint::Length(1)],
    )
    .areas(frame.area());

    let map = &world.map;
    let mut lines: Vec<Line> = (0..map.height)
        .map(|y| {
            let spans: Vec<Span> = (0..map.width)
                .map(|x| {
                    let cell = map.get(crate::map::Pos { x, y }).unwrap_or(Cell::Empty);
                    let (glyph, color) = cell_glyph(cell);
                    Span::styled(glyph.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    // Overlay robots on top of the map cells.
    for (&id, &pos) in &world.robot_positions {
        if pos.x < 0 || pos.x >= map.width || pos.y < 0 || pos.y >= map.height {
            continue;
        }
        if let Some(&kind) = world.robot_kinds.get(&id) {
            let (glyph, color) = robot_glyph(kind);
            lines[pos.y as usize].spans[pos.x as usize] =
                Span::styled(glyph.to_string(), Style::default().fg(color));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("resource-sim")),
        map_area,
    );

    let status = format!(
        "energy: {}  crystal: {}  (any key to quit)",
        world.energy_collected, world.crystal_collected
    );
    frame.render_widget(Paragraph::new(status), status_area);
}
