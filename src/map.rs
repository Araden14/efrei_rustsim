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

const RESOURCES_PER_KIND: u32 = 10;
const RESOURCE_QTY_RANGE: std::ops::RangeInclusive<u32> = 50..=200;
const OBSTACLE_THRESHOLD: f64 = 0.25;
const NOISE_SCALE: f64 = 0.12;

impl Map {
    pub fn generate(width: i32, height: i32, seed: u32) -> Self {
        use noise::{NoiseFn, Perlin};

        let base = Pos { x: width / 2, y: height / 2 };
        let perlin = Perlin::new(seed);
        let mut cells: Vec<Cell> = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let n = perlin.get([x as f64 * NOISE_SCALE, y as f64 * NOISE_SCALE]);
                    if n > OBSTACLE_THRESHOLD { Cell::Obstacle } else { Cell::Empty }
                })
            })
            .collect();

        // keep the base and the tile ring around it walkable
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = Pos { x: base.x + dx, y: base.y + dy };
                if p.x >= 0 && p.y >= 0 && p.x < width && p.y < height {
                    cells[(p.y * width + p.x) as usize] = Cell::Empty;
                }
            }
        }
        cells[(base.y * width + base.x) as usize] = Cell::Base;

        let mut map = Map { width, height, cells };
        map.scatter_resources(seed);
        map
    }

    fn scatter_resources(&mut self, seed: u32) {
        use rand::{Rng, SeedableRng, rngs::StdRng};

        let mut rng = StdRng::seed_from_u64(u64::from(seed) ^ 0x5EED);
        for kind in [ResourceKind::Energy, ResourceKind::Crystal] {
            let mut placed = 0;
            let mut attempts = 0;
            while placed < RESOURCES_PER_KIND && attempts < RESOURCES_PER_KIND * 100 {
                attempts += 1;
                let idx = rng.random_range(0..self.cells.len());
                if self.cells[idx] == Cell::Empty {
                    let qty = rng.random_range(RESOURCE_QTY_RANGE.clone());
                    self.cells[idx] = Cell::Resource(kind, qty);
                    placed += 1;
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_places_base_obstacles_and_resources() {
        let map = Map::generate(60, 30, 42);
        assert_eq!(map.get(map.base_pos()), Some(Cell::Base));

        let mut obstacles = 0;
        let mut energy = 0;
        let mut crystal = 0;
        for y in 0..map.height {
            for x in 0..map.width {
                match map.get(Pos { x, y }).unwrap() {
                    Cell::Obstacle => obstacles += 1,
                    Cell::Resource(ResourceKind::Energy, qty) => {
                        assert!((50..=200).contains(&qty));
                        energy += 1;
                    }
                    Cell::Resource(ResourceKind::Crystal, qty) => {
                        assert!((50..=200).contains(&qty));
                        crystal += 1;
                    }
                    _ => {}
                }
            }
        }
        assert!(obstacles > 0, "expected some obstacles from perlin noise");
        assert_eq!(energy, RESOURCES_PER_KIND);
        assert_eq!(crystal, RESOURCES_PER_KIND);
    }
}
