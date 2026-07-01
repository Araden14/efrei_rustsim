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
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

/// Simulation tick duration shared by all robot tasks.
const ROBOT_TICK_MS: u64 = 200;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Clone, Debug)]
pub enum RobotMessage {
    Discovered { pos: Pos, cell: Cell },
    Collected { kind: ResourceKind, amount: u32 },
    /// Sent by a collector after it unloads at base or drops a stale target,
    /// signalling to the base that it is free for a new dispatch assignment.
    CollectorIdle,
}

pub struct Robot {
    pub id: usize,
    pub kind: RobotKind,
    pub pos: Pos,
    /// Cells discovered by this robot. Only populated for scouts; collectors leave this empty.
    pub known_cells: HashSet<Pos>,
    /// Channel to send discoveries and collection events to the base.
    pub tx: Sender<RobotMessage>,
    /// Resource the collector is currently carrying (None for scouts or idle collectors).
    pub carrying: Option<ResourceKind>,
}

impl Robot {
    pub fn new(id: usize, kind: RobotKind, pos: Pos, tx: Sender<RobotMessage>) -> Self {
        Robot {
            id,
            kind,
            pos,
            known_cells: HashSet::new(),
            tx,
            carrying: None,
        }
    }

    /// Moves the scout one step in a random walkable direction, then scans the 8
    /// surrounding cells for new discoveries. Returns `(moved, messages)` where
    /// `moved` is `false` when all four cardinal directions are blocked.
    pub fn step_scout(&mut self, world: &SharedWorld) -> (bool, Vec<RobotMessage>) {
        let moved = self.move_randomly(world);
        let messages = self.scan_neighbors(world);
        (moved, messages)
    }

    /// Picks a random walkable cardinal direction and moves one step.
    /// Returns `true` if the scout moved, `false` if all directions were blocked.
    fn move_randomly(&mut self, world: &SharedWorld) -> bool {
        let mut dirs: Vec<(i32, i32)> = DIRECTIONS.to_vec();
        dirs.shuffle(&mut rand::rng());

        for (dx, dy) in dirs {
            let candidate = self.pos.offset(dx, dy);
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

    /// Scans the 8 surrounding cells. For each position not yet in `known_cells`,
    /// records it locally and returns a `Discovered` message for the base.
    /// Out-of-bounds neighbors are silently skipped and never marked as known.
    fn scan_neighbors(&mut self, world: &SharedWorld) -> Vec<RobotMessage> {
        let mut messages = Vec::new();

        for (dx, dy) in NEIGHBORS {
            let neighbor = self.pos.offset(dx, dy);

            if self.known_cells.contains(&neighbor) {
                continue;
            }

            if let Some(cell) = world.map.get(neighbor) {
                self.known_cells.insert(neighbor);
                messages.push(RobotMessage::Discovered { pos: neighbor, cell });
            }
            // None means out of bounds -> skip silently, don't mark as known.
        }

        messages
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectorAction {
    Idle,
    /// The assigned target is no longer a valid resource; drop it and notify the base.
    ClearTargetAndIdle,
    Move(Pos),
    MoveAndCollect { next: Pos, resource_pos: Pos },
    Collect(Pos),
    Unload,
}

fn plan_collector_step(robot: &Robot, world: &SharedWorld) -> CollectorAction {
    debug_assert_eq!(robot.kind, RobotKind::Collector);

    if robot.carrying.is_some() {
        let base_pos = world.map.base_pos();
        if robot.pos == base_pos {
            return CollectorAction::Unload;
        }

        return next_step_toward(robot.pos, base_pos, world)
            .map(CollectorAction::Move)
            .unwrap_or(CollectorAction::Idle);
    }

    // Look up the target assigned by the base dispatcher.
    // If no target has been assigned yet, wait idly.
    let Some(&target_pos) = world.collector_targets.get(&robot.id) else {
        return CollectorAction::Idle;
    };

    // Verify the target is still a live resource in the known map.
    // If it has been depleted or is no longer known, drop it and notify the base.
    match world.known_cells.get(&target_pos) {
        Some(Cell::Resource(_, amount)) if *amount > 0 => {}
        _ => return CollectorAction::ClearTargetAndIdle,
    }

    let Some(next_step) = next_step_toward(robot.pos, target_pos, world) else {
        // No path yet (unexplored terrain between here and target); wait.
        return CollectorAction::Idle;
    };

    if is_adjacent_or_same(robot.pos, target_pos) {
        CollectorAction::Collect(target_pos)
    } else if is_adjacent_or_same(next_step, target_pos) {
        CollectorAction::MoveAndCollect {
            next: next_step,
            resource_pos: target_pos,
        }
    } else {
        CollectorAction::Move(next_step)
    }
}

fn apply_collector_action(
    robot: &mut Robot,
    world: &mut SharedWorld,
    action: CollectorAction,
) -> Vec<RobotMessage> {
    match action {
        CollectorAction::Idle => Vec::new(),
        CollectorAction::ClearTargetAndIdle => {
            world.free_collector(robot.id);
            vec![RobotMessage::CollectorIdle]
        }
        CollectorAction::Move(next) => {
            robot.pos = next;
            Vec::new()
        }
        CollectorAction::MoveAndCollect { next, resource_pos } => {
            robot.pos = next;
            robot
                .collect_one_unit(resource_pos, world)
                .into_iter()
                .collect()
        }
        CollectorAction::Collect(resource_pos) => robot
            .collect_one_unit(resource_pos, world)
            .into_iter()
            .collect(),
        CollectorAction::Unload => {
            robot.carrying = None;
            world.free_collector(robot.id);
            vec![RobotMessage::CollectorIdle]
        }
    }
}

/// BFS from `from` toward `to` through known walkable cells.
/// Returns the first step to take, or `None` if no path exists yet.
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

    // Walk the came_from chain backward from `to` to find the first step after `from`.
    let mut current = to;
    while let Some(&previous) = came_from.get(&current) {
        if previous == from {
            return Some(current);
        }
        current = previous;
    }

    None
}

fn cardinal_neighbors(pos: Pos) -> impl Iterator<Item = Pos> {
    DIRECTIONS.into_iter().map(move |(dx, dy)| pos.offset(dx, dy))
}

fn is_known_walkable(pos: Pos, goal: Pos, world: &SharedWorld) -> bool {
    if world.map.get(pos).is_none() {
        return false;
    }

    if pos == goal {
        return matches!(
            world.known_cells.get(&pos),
            Some(Cell::Empty | Cell::Resource(_, _) | Cell::Base)
        )
        // The base is always a valid destination even before scouts have discovered it.
        || matches!(world.map.get(pos), Some(Cell::Base));
    }

    matches!(
        world.known_cells.get(&pos),
        Some(Cell::Empty | Cell::Resource(_, _) | Cell::Base)
    )
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
///   4. Sleep ROBOT_TICK_MS, or break if permanently stuck (obstacles are immovable)
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

        if moved {
            tokio::time::sleep(Duration::from_millis(ROBOT_TICK_MS)).await;
        } else {
            // Obstacles are permanent — stuck once means stuck forever. Stop the task.
            break;
        }
    }
}

/// Async task that drives a single collector for the lifetime of the simulation.
pub async fn run_collector(mut robot: Robot, world: Arc<RwLock<SharedWorld>>) {
    loop {
        let action = {
            let world_guard = world.read().await;
            plan_collector_step(&robot, &world_guard)
        };

        let messages = {
            let mut world_guard = world.write().await;
            let messages = apply_collector_action(&mut robot, &mut world_guard, action);
            world_guard.robot_positions.insert(robot.id, robot.pos);
            messages
        };

        for msg in messages {
            let _ = robot.tx.send(msg).await;
        }

        tokio::time::sleep(Duration::from_millis(ROBOT_TICK_MS)).await;
    }
}
