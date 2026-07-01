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

const RESOURCES_PER_KIND: usize = 10;
const OBSTACLE_THRESHOLD: f64 = 0.25;
const NOISE_SCALE: f64 = 0.12;

impl Map {
    pub fn generate(width: i32, height: i32, seed: u32) -> Self {
        use noise::{NoiseFn, Perlin};

        let base = Self::base_pos_for(width, height);
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
        use rand::{Rng, SeedableRng, rngs::StdRng, seq::IteratorRandom};

        let mut rng = StdRng::seed_from_u64(u64::from(seed) ^ 0x5EED);
        for kind in [ResourceKind::Energy, ResourceKind::Crystal] {
            let empty_indices = self
                .cells
                .iter()
                .enumerate()
                .filter(|(_, c)| **c == Cell::Empty)
                .map(|(i, _)| i)
                .choose_multiple(&mut rng, RESOURCES_PER_KIND);
            for idx in empty_indices {
                let qty = rng.random_range(10..=20);
                self.cells[idx] = Cell::Resource(kind, qty);
            }
        }
    }

    pub fn get(&self, pos: Pos) -> Option<Cell> {
        if pos.x < 0 || pos.y < 0 || pos.x >= self.width || pos.y >= self.height {
            return None;
        }
        Some(self.cells[(pos.y * self.width + pos.x) as usize])
    }

    pub fn set(&mut self, pos: Pos, cell: Cell) {
        if pos.x >= 0 && pos.y >= 0 && pos.x < self.width && pos.y < self.height {
            self.cells[(pos.y * self.width + pos.x) as usize] = cell;
        }
    }

    pub fn base_pos(&self) -> Pos {
        Self::base_pos_for(self.width, self.height)
    }

    fn base_pos_for(width: i32, height: i32) -> Pos {
        Pos { x: width / 2, y: height / 2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_places_base_obstacles_and_resources() {
        let map = Map::generate(60, 30, 42);
        assert_eq!(map.get(Map::base_pos_for(map.width, map.height)), Some(Cell::Base));

        let mut obstacles = 0usize;
        let mut energy = 0usize;
        let mut crystal = 0usize;
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