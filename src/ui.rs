use crate::map::{Cell, Pos};
use crate::robot::RobotKind;
use crate::world::SharedWorld;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::collections::HashMap;

/// Returns the character and color used to draw a given map cell.
fn cell_glyph(cell: Cell) -> (char, Color) {
    match cell {
        Cell::Empty => (' ', Color::Reset),
        Cell::Obstacle => ('O', Color::LightCyan),
        Cell::Resource(crate::map::ResourceKind::Energy, _) => ('E', Color::Yellow),
        Cell::Resource(crate::map::ResourceKind::Crystal, _) => ('C', Color::LightMagenta),
        Cell::Base => ('#', Color::LightGreen),
    }
}

/// Returns the character and color used to draw a robot of a given kind.
fn robot_glyph(kind: RobotKind) -> (char, Color) {
    match kind {
        RobotKind::Scout => ('x', Color::Red),
        RobotKind::Collector => ('o', Color::Magenta),
    }
}

pub fn render(frame: &mut Frame, world: &SharedWorld) {
    let [map_area, status_area] =
        Layout::new(Direction::Vertical, [Constraint::Min(0), Constraint::Length(1)])
            .areas(frame.area());

    let map = &world.map;

    // Build a Pos -> RobotKind lookup so we can overlay robots on top of cells
    // without scanning the whole robot list for each cell.
    let mut robot_at: HashMap<Pos, RobotKind> = HashMap::new();
    for (id, pos) in &world.robot_positions {
        if let Some(kind) = world.robot_kinds.get(id) {
            robot_at.insert(*pos, *kind);
        }
    }

    let lines: Vec<Line> = (0..map.height)
        .map(|y| {
            let spans: Vec<Span> = (0..map.width)
                .map(|x| {
                    let pos = Pos { x, y };
                    // A robot on a cell takes visual priority over the cell itself.
                    if let Some(kind) = robot_at.get(&pos) {
                        let (glyph, color) = robot_glyph(*kind);
                        return Span::styled(glyph.to_string(), Style::default().fg(color));
                    }
                    let cell = map.get(pos).unwrap_or(Cell::Empty);
                    let (glyph, color) = cell_glyph(cell);
                    Span::styled(glyph.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("resource-sim")),
        map_area,
    );

    // Count resources still on the map (not yet collected).
    let mut energy_remaining: u32 = 0;
    let mut crystal_remaining: u32 = 0;
    for y in 0..map.height {
        for x in 0..map.width {
            if let Some(Cell::Resource(kind, amount)) = map.get(Pos { x, y }) {
                match kind {
                    crate::map::ResourceKind::Energy => energy_remaining += amount,
                    crate::map::ResourceKind::Crystal => crystal_remaining += amount,
                }
            }
        }
    }

    let status = format!(
        "robots: {}  |  collected  E: {}  C: {}  |  remaining  E: {}  C: {}  |  (any key to quit)",
        world.robot_positions.len(),
        world.energy_collected,
        world.crystal_collected,
        energy_remaining,
        crystal_remaining,
    );
    frame.render_widget(Paragraph::new(status), status_area);
}