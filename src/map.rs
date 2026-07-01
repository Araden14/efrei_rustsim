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
    pub fn generate(width: i32, height: i32, seed: u32, cfg: &crate::config::MapConfig) -> Self {
        use noise::{NoiseFn, Perlin};

        let base = Self::base_pos_for(width, height);
        let perlin = Perlin::new(seed);
        let mut cells: Vec<Cell> = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let n = perlin.get([x as f64 * cfg.noise_scale, y as f64 * cfg.noise_scale]);
                    if n > cfg.obstacle_threshold {
                        Cell::Obstacle
                    } else {
                        Cell::Empty
                    }
                })
            })
            .collect();

        // keep the base and the tile ring around it walkable
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = Pos {
                    x: base.x + dx,
                    y: base.y + dy,
                };
                if p.x >= 0 && p.y >= 0 && p.x < width && p.y < height {
                    cells[(p.y * width + p.x) as usize] = Cell::Empty;
                }
            }
        }
        cells[(base.y * width + base.x) as usize] = Cell::Base;

        let mut map = Map {
            width,
            height,
            cells,
        };
        map.scatter_resources(seed, cfg);
        map
    }

    fn scatter_resources(&mut self, seed: u32, cfg: &crate::config::MapConfig) {
        use rand::{rngs::StdRng, seq::IteratorRandom, Rng, SeedableRng};

        let mut rng = StdRng::seed_from_u64(u64::from(seed) ^ 0x5EED);
        for kind in [ResourceKind::Energy, ResourceKind::Crystal] {
            let empty_indices = self
                .cells
                .iter()
                .enumerate()
                .filter(|(_, c)| **c == Cell::Empty)
                .map(|(i, _)| i)
                .choose_multiple(&mut rng, cfg.resources_per_kind);
            for idx in empty_indices {
                let qty = rng.random_range(cfg.resource_qty_min..=cfg.resource_qty_max);
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
        Pos {
            x: width / 2,
            y: height / 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MapConfig;

    #[test]
    fn generate_places_base_obstacles_and_resources() {
        let cfg = MapConfig::default();
        let map = Map::generate(60, 30, 42, &cfg);
        assert_eq!(
            map.get(Map::base_pos_for(map.width, map.height)),
            Some(Cell::Base)
        );

        let mut obstacles = 0usize;
        let mut energy = 0usize;
        let mut crystal = 0usize;
        for y in 0..map.height {
            for x in 0..map.width {
                match map.get(Pos { x, y }).unwrap() {
                    Cell::Obstacle => obstacles += 1,
                    Cell::Resource(ResourceKind::Energy, qty) => {
                        assert!(
                            (cfg.resource_qty_min..=cfg.resource_qty_max).contains(&qty),
                            "energy qty {qty} out of range"
                        );
                        energy += 1;
                    }
                    Cell::Resource(ResourceKind::Crystal, qty) => {
                        assert!(
                            (cfg.resource_qty_min..=cfg.resource_qty_max).contains(&qty),
                            "crystal qty {qty} out of range"
                        );
                        crystal += 1;
                    }
                    _ => {}
                }
            }
        }
        assert!(obstacles > 0, "expected some obstacles from perlin noise");
        assert_eq!(energy, cfg.resources_per_kind);
        assert_eq!(crystal, cfg.resources_per_kind);
    }
}
