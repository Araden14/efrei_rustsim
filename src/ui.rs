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

fn cell_glyph(cell: Cell) -> (char, Color) {
    match cell {
        Cell::Empty => (' ', Color::Reset),
        Cell::Obstacle => ('O', Color::LightCyan),
        Cell::Resource(crate::map::ResourceKind::Energy, _) => ('E', Color::Yellow),
        Cell::Resource(crate::map::ResourceKind::Crystal, _) => ('C', Color::LightMagenta),
        Cell::Base => ('#', Color::LightGreen),
    }
}

pub fn render(frame: &mut Frame, world: &SharedWorld) {
    let [map_area, status_area] =
        Layout::new(Direction::Vertical, [Constraint::Min(0), Constraint::Length(1)])
            .areas(frame.area());

    let map = &world.map;
    let lines: Vec<Line> = (0..map.height)
        .map(|y| {
            let spans: Vec<Span> = (0..map.width)
                .map(|x| {
                    let pos = Pos { x, y };
                    let robot_here = |kind| {
                        world.robot_positions.iter().any(|(id, p)| {
                            *p == pos && world.robot_kinds.get(id) == Some(&kind)
                        })
                    };
                    let (glyph, color) = if robot_here(RobotKind::Scout) {
                        ('x', Color::White)
                    } else if robot_here(RobotKind::Collector) {
                        ('o', Color::LightBlue)
                    } else {
                        cell_glyph(map.get(pos).unwrap_or(Cell::Empty))
                    };
                    Span::styled(glyph.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(Block::bordered().title("resource-sim")), map_area);

    let status = format!(
        "energy: {}  crystal: {}  (any key to quit)",
        world.energy_collected, world.crystal_collected
    );
    frame.render_widget(Paragraph::new(status), status_area);
}
