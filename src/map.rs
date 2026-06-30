#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceKind {
    Energy,
    Crystal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Empty,
    Obstacle,
    Resource(ResourceKind, u32),
    Base,
}

pub struct Map {
    pub width: i32,
    pub height: i32,
    cells: Vec<Cell>,
}

impl Map {
    // ponytail: no Perlin noise / resource placement yet (Map Generation phase) — flat empty map with a centered base.
    pub fn empty(width: i32, height: i32) -> Self {
        let mut cells = vec![Cell::Empty; (width * height) as usize];
        let base = Pos { x: width / 2, y: height / 2 };
        cells[(base.y * width + base.x) as usize] = Cell::Base;
        Map { width, height, cells }
    }

    pub fn get(&self, pos: Pos) -> Option<Cell> {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.width || pos.y >= self.height {
            return None;
        }
        Some(self.cells[(pos.y * self.width + pos.x) as usize])
    }

    pub fn base_pos(&self) -> Pos {
        Pos { x: self.width / 2, y: self.height / 2 }
    }
}
