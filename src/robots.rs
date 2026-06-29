use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use rand::seq::IteratorRandom;
use crate::map::{Cell, Map, Pos, ResourceKind};
use crate::world::SharedWorld;

const NEIGHBORS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

// ponytail: full BFS per step; replace with cached path if perf matters
fn bfs_next_step(map: &Map, from: Pos, to: Pos) -> Option<Pos> {
    if from == to { return None; }
    let mut queue = VecDeque::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    queue.push_back(from);
    came_from.insert(from, from);
    while let Some(cur) = queue.pop_front() {
        if cur == to {
            let mut step = cur;
            loop {
                let prev = came_from[&step];
                if prev == from { return Some(step); }
                step = prev;
            }
        }
        for &(dx, dy) in &NEIGHBORS {
            let next = Pos { x: cur.x + dx, y: cur.y + dy };
            if !came_from.contains_key(&next) {
                if let Some(cell) = map.get(next) {
                    if cell != Cell::Obstacle {
                        came_from.insert(next, cur);
                        queue.push_back(next);
                    }
                }
            }
        }
    }
    None
}

pub async fn scout_loop(world: Arc<RwLock<SharedWorld>>, start: Pos, id: usize) {
    let mut pos = start;
    loop {
        let next_pos = {
            let w = world.read().await;
            let mut rng = rand::rng();
            NEIGHBORS
                .iter()
                .map(|&(dx, dy)| Pos { x: pos.x + dx, y: pos.y + dy })
                .filter(|&p| matches!(w.map.get(p), Some(c) if c != Cell::Obstacle))
                .choose(&mut rng)
        };

        if let Some(p) = next_pos {
            pos = p;
            let mut w = world.write().await;
            w.scout_positions[id] = pos;
            if let Some(Cell::Resource(kind, _)) = w.map.get(pos) {
                if !w.known_resources.iter().any(|&(rp, _)| rp == pos) {
                    w.known_resources.push((pos, kind));
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

pub async fn collector_loop(world: Arc<RwLock<SharedWorld>>, start: Pos, id: usize) {
    let mut pos = start;
    let mut carrying: Option<ResourceKind> = None;
    let mut target: Option<Pos> = None;

    loop {
        let (next_step, base_pos) = {
            let w = world.read().await;

            if carrying.is_none() && target.is_none() {
                target = w.known_resources
                    .iter()
                    .min_by_key(|&&(rp, _)| manhattan(pos, rp))
                    .map(|&(rp, _)| rp);
            }

            let dest = if carrying.is_some() { Some(w.base_pos) } else { target };
            let next_step = dest.and_then(|d| bfs_next_step(&w.map, pos, d));
            (next_step, w.base_pos)
        };

        if let Some(p) = next_step {
            pos = p;
        }

        {
            let mut w = world.write().await;
            w.collector_positions[id] = pos;

            if carrying.is_some() && pos == base_pos {
                match carrying {
                    Some(ResourceKind::Energy) => w.energy_collected += 1,
                    Some(ResourceKind::Crystal) => w.crystal_collected += 1,
                    None => {}
                }
                carrying = None;
            } else if let Some(tgt) = target {
                if pos == tgt {
                    match w.map.get(pos) {
                        Some(Cell::Resource(kind, qty)) => {
                            carrying = Some(kind);
                            let new_qty = qty.saturating_sub(1);
                            w.map.set(pos, if new_qty == 0 { Cell::Empty } else { Cell::Resource(kind, new_qty) });
                        }
                        _ => {}
                    }
                    w.known_resources.retain(|&(rp, _)| rp != pos);
                    target = None;
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}
