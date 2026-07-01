use crate::map::{Cell, Pos, ResourceKind};
use crate::world::SharedWorld;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

const NEIGHBORS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Clone, Debug)]
pub enum RobotMessage {
    Discovered { pos: Pos, cell: Cell },
    Collected { kind: ResourceKind, amount: u32 },
    CollectorIdle(usize),
}

pub struct Robot {
    pub id: usize,
    pub pos: Pos,
    pub known_cells: HashSet<Pos>,
    pub tx: Sender<RobotMessage>,
    pub carrying: Option<ResourceKind>,
}

impl Robot {
    pub fn new_scout(id: usize, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None,
        }
    }

    pub fn new_collector(id: usize, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None,
        }
    }

    pub fn step_scout(&mut self, world: &SharedWorld) -> (bool, Vec<RobotMessage>) {
        let moved = self.move_randomly(world);
        let messages = self.scan_neighbors(world);
        (moved, messages)
    }

    fn move_randomly(&mut self, world: &SharedWorld) -> bool {
        let mut dirs: Vec<(i32, i32)> = DIRECTIONS.to_vec();
        dirs.shuffle(&mut rand::rng());

        for (dx, dy) in dirs {
            let candidate = Pos {
                x: self.pos.x + dx,
                y: self.pos.y + dy,
            };
            match world.map.get(candidate) {
                Some(Cell::Obstacle) | None => continue,
                Some(_) => {
                    self.pos = candidate;
                    return true;
                }
            }
        }
        false
    }

    fn scan_neighbors(&mut self, world: &SharedWorld) -> Vec<RobotMessage> {
        let mut messages = Vec::new();

        for (dx, dy) in NEIGHBORS {
            let neighbor = Pos {
                x: self.pos.x + dx,
                y: self.pos.y + dy,
            };

            if self.known_cells.contains(&neighbor) {
                continue;
            }

            if let Some(cell) = world.map.get(neighbor) {
                self.known_cells.insert(neighbor);
                messages.push(RobotMessage::Discovered {
                    pos: neighbor,
                    cell,
                });
            }
        }

        messages
    }
}

impl Robot {
    fn collect_one_unit(
        &mut self,
        resource_pos: Pos,
        world: &mut SharedWorld,
    ) -> Option<RobotMessage> {
        let Some(Cell::Resource(kind, amount)) = world.map.get(resource_pos) else {
            world.known_cells.remove(&resource_pos);
            return None;
        };
        if amount == 0 {
            world.known_cells.remove(&resource_pos);
            return None;
        }

        let updated_cell = if amount == 1 {
            Cell::Empty
        } else {
            Cell::Resource(kind, amount - 1)
        };
        world.map.set(resource_pos, updated_cell);
        world.known_cells.insert(resource_pos, updated_cell);

        self.carrying = Some(kind);
        Some(RobotMessage::Collected { kind, amount: 1 })
    }
}

fn step_collector(robot: &mut Robot, world: &mut SharedWorld) -> Vec<RobotMessage> {
    if robot.carrying.is_some() {
        let base = world.map.base_pos();
        if robot.pos == base {
            robot.carrying = None;
            world.collector_targets.remove(&robot.id);
            return vec![RobotMessage::CollectorIdle(robot.id)];
        }
        if let Some(next) = next_step_toward(robot.pos, base, world) {
            robot.pos = next;
        }
        return Vec::new();
    }

    let Some(&target) = world.collector_targets.get(&robot.id) else {
        return Vec::new();
    };
    if !matches!(world.known_cells.get(&target), Some(Cell::Resource(_, amount)) if *amount > 0) {
        world.collector_targets.remove(&robot.id);
        return vec![RobotMessage::CollectorIdle(robot.id)];
    }

    if !is_adjacent_or_same(robot.pos, target) {
        let Some(next) = next_step_toward(robot.pos, target, world) else {
            return Vec::new();
        };
        robot.pos = next;
    }

    if is_adjacent_or_same(robot.pos, target) {
        robot.collect_one_unit(target, world).into_iter().collect()
    } else {
        Vec::new()
    }
}

fn next_step_toward(from: Pos, to: Pos, world: &SharedWorld) -> Option<Pos> {
    if from == to {
        return Some(from);
    }

    let mut frontier = VecDeque::from([from]);
    let mut visited = HashSet::from([from]);
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();

    while let Some(current) = frontier.pop_front() {
        if current == to {
            break;
        }

        for next in cardinal_neighbors(current) {
            if visited.contains(&next) || !is_known_walkable(next, to, world) {
                continue;
            }

            visited.insert(next);
            came_from.insert(next, current);
            frontier.push_back(next);
        }
    }

    if !came_from.contains_key(&to) {
        return None;
    }

    let mut current = to;
    while let Some(previous) = came_from.get(&current).copied() {
        if previous == from {
            return Some(current);
        }
        current = previous;
    }

    None
}

fn cardinal_neighbors(pos: Pos) -> impl Iterator<Item = Pos> {
    DIRECTIONS.into_iter().map(move |(dx, dy)| Pos {
        x: pos.x + dx,
        y: pos.y + dy,
    })
}

fn is_known_walkable(pos: Pos, goal: Pos, world: &SharedWorld) -> bool {
    matches!(
        world.known_cells.get(&pos),
        Some(Cell::Empty | Cell::Resource(_, _) | Cell::Base)
    ) || (pos == goal && matches!(world.map.get(pos), Some(Cell::Base)))
}

fn is_adjacent_or_same(a: Pos, b: Pos) -> bool {
    (a.x - b.x).abs() + (a.y - b.y).abs() <= 1
}

pub async fn run_scout(mut robot: Robot, world: Arc<RwLock<SharedWorld>>) {
    loop {
        let (moved, messages) = {
            let world_guard = world.read().await;
            robot.step_scout(&world_guard)
        };

        // Publish the new position (write lock released immediately after)
        {
            let mut world_guard = world.write().await;
            world_guard.robot_positions.insert(robot.id, robot.pos);
        }

        for msg in messages {
            let _ = robot.tx.send(msg).await;
        }

        if moved {
            tokio::time::sleep(Duration::from_millis(200)).await;
        } else {
            break;
        }
    }
}

pub async fn run_collector(mut robot: Robot, world: Arc<RwLock<SharedWorld>>) {
    loop {
        let messages = {
            let mut world_guard = world.write().await;
            let messages = step_collector(&mut robot, &mut world_guard);
            world_guard.robot_positions.insert(robot.id, robot.pos);
            messages
        };

        for msg in messages {
            let _ = robot.tx.send(msg).await;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
