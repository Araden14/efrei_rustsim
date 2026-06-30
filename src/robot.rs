use crate::map::{Cell, Pos, ResourceKind};
use crate::world::SharedWorld;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;

// Cardinal directions for movement: N, S, W, E
const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

// Moore neighborhood (8 surrounding cells) for discovery scanning
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
}

pub struct Robot {
    pub id: usize,
    pub kind: RobotKind,
    pub pos: Pos,
    pub known_cells: HashSet<Pos>,
    /// Channel to send discoveries and collection events to the base.
    pub tx: Sender<RobotMessage>,
    /// Resource the collector is currently carrying (None for scouts or idle collectors).
    pub carrying: Option<ResourceKind>,
}

impl Robot {
    pub fn new_scout(id: usize, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            kind: RobotKind::Scout,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None, // scouts never carry resources
        }
    }

    pub fn new_collector(id: usize, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            kind: RobotKind::Collector,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None,
        }
    }

    /**
     * Moves the scout one step in a random walkable direction, then scans the 8 surrounding cells for new discoveries.
     * Returns `(moved, messages)` where `moved` is `false` when all four cardinal
     * directions are blocked. The caller owns the decision of what to do when stuck.
     */
    pub fn step_scout(&mut self, world: &SharedWorld) -> (bool, Vec<RobotMessage>) {
        let moved = self.move_randomly(world);
        let messages = self.scan_neighbors(world);
        (moved, messages)
    }

    /**
     * Picks a random walkable cardinal direction and moves one step.
     * Returns `true` if the scout moved, `false` if all directions were blocked.
     */
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

    /**
     * Scans the 8 surrounding cells.
     * For each position not yet in 'known_cells', records it locally and returns a 'Discovered' message for the base.
     * Out-of-bounds neighbors are silently skipped and never marked as known.
     */
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
            // None means out of bounds -> skip silently, don't mark as known.
        }

        messages
    }
}

impl Robot {
    pub fn step_collector(&mut self, world: &mut SharedWorld) {
        if self.kind != RobotKind::Collector {
            return;
        }

        if self.carrying.is_some() {
            let base_pos = world.map.base_pos();
            if self.pos == base_pos {
                self.carrying = None;
                return;
            }

            if let Some(next) = next_step_toward(self.pos, base_pos, world) {
                self.pos = next;
            }

            if self.pos == base_pos {
                self.carrying = None;
            }
            return;
        }

        let Some(resource_pos) = nearest_known_resource(self.pos, world) else {
            return;
        };

        if !is_adjacent_or_same(self.pos, resource_pos) {
            if let Some(next) = next_step_toward(self.pos, resource_pos, world) {
                self.pos = next;
            }
        }

        if is_adjacent_or_same(self.pos, resource_pos) {
            self.collect_one_unit(resource_pos, world);
        }
    }

    fn collect_one_unit(&mut self, resource_pos: Pos, world: &mut SharedWorld) {
        let Some(Cell::Resource(kind, amount)) = world.map.get(resource_pos) else {
            world.known_cells.remove(&resource_pos);
            return;
        };
        if amount == 0 {
            world.known_cells.remove(&resource_pos);
            return;
        }

        let updated_cell = if amount == 1 {
            Cell::Empty
        } else {
            Cell::Resource(kind, amount - 1)
        };
        world.map.set(resource_pos, updated_cell);
        world.known_cells.insert(resource_pos, updated_cell);

        self.carrying = Some(kind);
        let _ = self
            .tx
            .try_send(RobotMessage::Collected { kind, amount: 1 });
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
            if visited.contains(&next) || !is_known_walkable(next, world) {
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

fn nearest_known_resource(from: Pos, world: &SharedWorld) -> Option<Pos> {
    world
        .known_cells
        .iter()
        .filter_map(|(pos, cell)| match cell {
            Cell::Resource(_, amount) if *amount > 0 => Some(*pos),
            _ => None,
        })
        .filter(|pos| *pos == from || next_step_toward(from, *pos, world).is_some())
        .min_by_key(|pos| manhattan_distance(from, *pos))
}

fn cardinal_neighbors(pos: Pos) -> impl Iterator<Item = Pos> {
    DIRECTIONS.into_iter().map(move |(dx, dy)| Pos {
        x: pos.x + dx,
        y: pos.y + dy,
    })
}

fn is_known_walkable(pos: Pos, world: &SharedWorld) -> bool {
    if world.map.get(pos).is_none() {
        return false;
    }

    !matches!(world.known_cells.get(&pos), Some(Cell::Obstacle))
}

fn is_adjacent_or_same(a: Pos, b: Pos) -> bool {
    manhattan_distance(a, b) <= 1
}

fn manhattan_distance(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Async task that drives a single scout for the lifetime of the simulation.
///
///   1. Read lock  → step_scout (movement + discovery)
///   2. Write lock → update robot_positions
///   3. No lock    → forward discovery messages to base via channel
///   4. Sleep 200ms, or break if permanently stuck (obstacles are immovable)
pub async fn run_scout(mut robot: Robot, world: Arc<RwLock<SharedWorld>>) {
    loop {
        // Compute movement + discoveries (read lock released before any await)
        let (moved, messages) = {
            let world_guard = world.read().await;
            robot.step_scout(&world_guard)
        };

        // Publish the new position (write lock released immediately after)
        {
            let mut world_guard = world.write().await;
            world_guard.robot_positions.insert(robot.id, robot.pos);
        }

        // Forward discoveries to the base — no lock held
        for msg in messages {
            let _ = robot.tx.send(msg).await;
        }

        // Do not force movement if scout is stuck
        if moved {
            tokio::time::sleep(Duration::from_millis(200)).await;
        } else {
            // Obstacles are permanent — stuck once means stuck forever. Stop the task.
            break;
        }
    }
}

/// Async task that drives a single collector for the lifetime of the simulation.
pub async fn run_collector(mut robot: Robot, world: Arc<RwLock<SharedWorld>>) {
    loop {
        {
            let mut world_guard = world.write().await;
            robot.step_collector(&mut world_guard);
            world_guard.robot_positions.insert(robot.id, robot.pos);
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
