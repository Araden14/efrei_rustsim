use crate::map::{Cell, Pos, ResourceKind};
use crate::world::SharedWorld;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

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
