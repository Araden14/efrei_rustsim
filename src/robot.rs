use crate::map::{Map, Pos, ResourceKind};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::mpsc::Sender;

const COLLECTOR_CAPACITY: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobotType {
    Scout,
    Collector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectorMode {
    Seeking,
    Returning,
}

#[derive(Clone, Debug)]
pub enum RobotMessage {
    Discovered {
        robot_id: u32,
        pos: Pos,
        kind: ResourceKind,
        amount: u32,
    },
    Collected {
        robot_id: u32,
        kind: ResourceKind,
        amount: u32,
    },
}

#[derive(Clone, Debug)]
pub struct Robot {
    pub id: u32,
    pub robot_type: RobotType,
    pub pos: Pos,
    pub base_pos: Pos,
    pub carrying: (u32, u32),
    pub known_resources: HashMap<Pos, ResourceKind>,
    pub mode: CollectorMode,
    tx: Sender<RobotMessage>,
}

impl Robot {
    pub fn new(
        id: u32,
        robot_type: RobotType,
        base_pos: Pos,
        start_pos: Pos,
        tx: Sender<RobotMessage>,
    ) -> Self {
        Robot {
            id,
            robot_type,
            pos: start_pos,
            base_pos,
            carrying: (0, 0),
            known_resources: HashMap::new(),
            mode: CollectorMode::Seeking,
            tx,
        }
    }

    pub fn sync_known_resources(&mut self, resources: &HashMap<Pos, ResourceKind>) {
        if self.robot_type == RobotType::Collector {
            self.known_resources = resources.clone();
        }
    }

    pub fn scout_next_pos(&self, map: &Map, occupied: &HashSet<Pos>) -> Pos {
        const DIRS: [(isize, isize); 8] = [
            (0, 1),
            (1, 0),
            (0, -1),
            (-1, 0),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
        ];

        let start = rand::random::<usize>() % DIRS.len();
        for offset in 0..DIRS.len() {
            let (dx, dy) = DIRS[(start + offset) % DIRS.len()];
            let Some(x) = self.pos.x.checked_add_signed(dx) else {
                continue;
            };
            let Some(y) = self.pos.y.checked_add_signed(dy) else {
                continue;
            };
            let next = Pos { x, y };

            if map.is_walkable_pos(next) && !occupied.contains(&next) {
                return next;
            }
        }

        self.pos
    }

    pub fn collector_next_pos(&self, map: &Map, occupied: &HashSet<Pos>) -> Pos {
        let target = match self.mode {
            CollectorMode::Returning => self.base_pos,
            CollectorMode::Seeking => self.closest_known_resource(),
        };

        let Some(target) = target else {
            return self.pos;
        };

        astar_next_step(map, self.pos, target, occupied).unwrap_or(self.pos)
    }

    pub fn discover_nearby_resources(&self, map: &Map) {
        for pos in scan_positions(self.pos) {
            let Some(cell) = map.get(pos) else {
                continue;
            };
            let Some(kind) = cell.resource_kind() else {
                continue;
            };

            let _ = self.tx.send(RobotMessage::Discovered {
                robot_id: self.id,
                pos,
                kind,
                amount: cell.resource_amount(),
            });
        }
    }

    pub fn try_collect(&mut self, map: &mut Map) {
        if self.robot_type != RobotType::Collector || self.mode != CollectorMode::Seeking {
            return;
        }

        let used_capacity = self.carrying.0 + self.carrying.1;
        if used_capacity >= COLLECTOR_CAPACITY {
            self.mode = CollectorMode::Returning;
            return;
        }

        let Some(expected_kind) = self.known_resources.get(&self.pos).copied() else {
            return;
        };
        let Some((kind, amount)) = map.take_resource(self.pos, COLLECTOR_CAPACITY - used_capacity)
        else {
            self.known_resources.remove(&self.pos);
            return;
        };

        if kind != expected_kind {
            self.known_resources.insert(self.pos, kind);
        }

        match kind {
            ResourceKind::Energy => self.carrying.0 += amount,
            ResourceKind::Crystal => self.carrying.1 += amount,
        }

        if map.get(self.pos).is_some_and(|cell| cell.resource_kind().is_none()) {
            self.known_resources.remove(&self.pos);
        }
        self.mode = CollectorMode::Returning;
    }

    pub fn try_deposit(&mut self) {
        if self.robot_type != RobotType::Collector || self.pos != self.base_pos {
            return;
        }

        if self.carrying.0 > 0 {
            let _ = self.tx.send(RobotMessage::Collected {
                robot_id: self.id,
                kind: ResourceKind::Energy,
                amount: self.carrying.0,
            });
        }
        if self.carrying.1 > 0 {
            let _ = self.tx.send(RobotMessage::Collected {
                robot_id: self.id,
                kind: ResourceKind::Crystal,
                amount: self.carrying.1,
            });
        }

        self.carrying = (0, 0);
        self.mode = CollectorMode::Seeking;
    }

    fn closest_known_resource(&self) -> Option<Pos> {
        self.known_resources
            .keys()
            .copied()
            .min_by_key(|pos| self.pos.manhattan_distance(*pos))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Node {
    pos: Pos,
    cost: usize,
    estimate: usize,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimate
            .cmp(&self.estimate)
            .then_with(|| other.cost.cmp(&self.cost))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn astar_next_step(
    map: &Map,
    start: Pos,
    goal: Pos,
    occupied: &HashSet<Pos>,
) -> Option<Pos> {
    if start == goal {
        return Some(start);
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    let mut cost_so_far: HashMap<Pos, usize> = HashMap::new();

    open.push(Node {
        pos: start,
        cost: 0,
        estimate: start.manhattan_distance(goal),
    });
    cost_so_far.insert(start, 0);

    while let Some(Node { pos, cost, .. }) = open.pop() {
        if pos == goal {
            return reconstruct_next_step(start, goal, &came_from);
        }

        if cost > *cost_so_far.get(&pos).unwrap_or(&usize::MAX) {
            continue;
        }

        for next in map.neighbors(pos) {
            if next != goal && occupied.contains(&next) {
                continue;
            }

            let next_cost = cost + 1;
            if next_cost >= *cost_so_far.get(&next).unwrap_or(&usize::MAX) {
                continue;
            }

            cost_so_far.insert(next, next_cost);
            came_from.insert(next, pos);
            open.push(Node {
                pos: next,
                cost: next_cost,
                estimate: next_cost + next.manhattan_distance(goal),
            });
        }
    }

    None
}

fn reconstruct_next_step(start: Pos, goal: Pos, came_from: &HashMap<Pos, Pos>) -> Option<Pos> {
    let mut current = goal;
    let mut previous = *came_from.get(&current)?;

    while previous != start {
        current = previous;
        previous = *came_from.get(&current)?;
    }

    Some(current)
}

fn scan_positions(center: Pos) -> impl Iterator<Item = Pos> {
    (-1isize..=1).flat_map(move |dy| {
        (-1isize..=1).filter_map(move |dx| {
            let x = center.x.checked_add_signed(dx)?;
            let y = center.y.checked_add_signed(dy)?;
            Some(Pos { x, y })
        })
    })
}
