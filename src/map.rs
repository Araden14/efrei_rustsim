use noise::{NoiseFn, Perlin};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn manhattan_distance(self, other: Pos) -> usize {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Energy,
    Crystal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Obstacle,
    Energy(u32),
    Crystal(u32),
}

impl Cell {
    pub fn resource_kind(self) -> Option<ResourceKind> {
        match self {
            Cell::Energy(_) => Some(ResourceKind::Energy),
            Cell::Crystal(_) => Some(ResourceKind::Crystal),
            _ => None,
        }
    }

    pub fn resource_amount(self) -> u32 {
        match self {
            Cell::Energy(amount) | Cell::Crystal(amount) => amount,
            _ => 0,
        }
    }
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Vec<Cell>>,
}

impl Map {
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let mut grid = vec![vec![Cell::Empty; width]; height];

        for y in 0..height {
            for x in 0..width {
                let nx = (x as f64) / (width as f64) * 4.0;
                let ny = (y as f64) / (height as f64) * 4.0;

                if perlin.get([nx, ny]) > 0.3 {
                    grid[y][x] = Cell::Obstacle;
                }
            }
        }

        let base = Pos {
            x: width / 2,
            y: height / 2,
        };
        clear_area(&mut grid, width, height, base, 2);

        place_resources(&mut grid, width, height, ResourceKind::Energy, 10);
        place_resources(&mut grid, width, height, ResourceKind::Crystal, 10);

        Map {
            width,
            height,
            grid,
        }
    }

    pub fn in_bounds(&self, pos: Pos) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    pub fn get(&self, pos: Pos) -> Option<Cell> {
        self.in_bounds(pos).then_some(self.grid[pos.y][pos.x])
    }

    pub fn set(&mut self, pos: Pos, cell: Cell) {
        if self.in_bounds(pos) {
            self.grid[pos.y][pos.x] = cell;
        }
    }

    pub fn is_walkable_pos(&self, pos: Pos) -> bool {
        self.get(pos).is_some_and(|cell| cell != Cell::Obstacle)
    }

    pub fn neighbors(&self, pos: Pos) -> impl Iterator<Item = Pos> + '_ {
        const DIRS: [(isize, isize); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

        DIRS.into_iter().filter_map(move |(dx, dy)| {
            let x = pos.x.checked_add_signed(dx)?;
            let y = pos.y.checked_add_signed(dy)?;
            let next = Pos { x, y };
            self.is_walkable_pos(next).then_some(next)
        })
    }

    pub fn take_resource(&mut self, pos: Pos, capacity: u32) -> Option<(ResourceKind, u32)> {
        let cell = self.get(pos)?;
        let kind = cell.resource_kind()?;
        let amount = cell.resource_amount().min(capacity);
        let remaining = cell.resource_amount() - amount;

        let new_cell = match (kind, remaining) {
            (_, 0) => Cell::Empty,
            (ResourceKind::Energy, amount) => Cell::Energy(amount),
            (ResourceKind::Crystal, amount) => Cell::Crystal(amount),
        };
        self.set(pos, new_cell);

        Some((kind, amount))
    }
}

fn clear_area(grid: &mut [Vec<Cell>], width: usize, height: usize, center: Pos, radius: usize) {
    let min_x = center.x.saturating_sub(radius);
    let max_x = (center.x + radius).min(width.saturating_sub(1));
    let min_y = center.y.saturating_sub(radius);
    let max_y = (center.y + radius).min(height.saturating_sub(1));

    for row in grid.iter_mut().take(max_y + 1).skip(min_y) {
        for cell in row.iter_mut().take(max_x + 1).skip(min_x) {
            *cell = Cell::Empty;
        }
    }
}

fn place_resources(
    grid: &mut [Vec<Cell>],
    width: usize,
    height: usize,
    kind: ResourceKind,
    count: usize,
) {
    for _ in 0..count {
        let x = rand::random::<usize>() % width;
        let y = rand::random::<usize>() % height;

        if grid[y][x] == Cell::Empty {
            let amount = 50 + (rand::random::<u32>() % 150);
            grid[y][x] = match kind {
                ResourceKind::Energy => Cell::Energy(amount),
                ResourceKind::Crystal => Cell::Crystal(amount),
            };
        }
    }
}
