use crate::base::Base;
use crate::map::{Map, Pos};
use crate::robot::{Robot, RobotType};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

pub struct World {
    pub map: Map,
    pub robots: Vec<Robot>,
    pub base: Base,
    pub tick: u64,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        let map = Map::new(width, height, 42);
        let base_pos = Pos {
            x: width / 2,
            y: height / 2,
        };
        let (tx, rx) = mpsc::channel();
        let base = Base::new(base_pos, rx);

        let spawns = spawn_positions(base_pos);
        let robots = vec![
            Robot::new(1, RobotType::Scout, base_pos, spawns[0], tx.clone()),
            Robot::new(2, RobotType::Scout, base_pos, spawns[1], tx.clone()),
            Robot::new(3, RobotType::Collector, base_pos, spawns[2], tx.clone()),
            Robot::new(4, RobotType::Collector, base_pos, spawns[3], tx),
        ];

        World {
            map,
            robots,
            base,
            tick: 0,
        }
    }

    pub fn update(&mut self) {
        self.base.drain_messages();
        self.remove_depleted_discoveries();

        for robot in &mut self.robots {
            robot.sync_known_resources(&self.base.discovered_resources);
            robot.discover_nearby_resources(&self.map);
            robot.try_deposit();
        }

        let proposals = self.propose_moves();
        self.apply_moves(proposals);

        for robot in &mut self.robots {
            robot.discover_nearby_resources(&self.map);
            robot.try_collect(&mut self.map);
            robot.try_deposit();
        }

        self.base.drain_messages();
        self.remove_depleted_discoveries();
        self.tick += 1;
    }

    fn propose_moves(&self) -> Vec<Pos> {
        self.robots
            .iter()
            .enumerate()
            .map(|(index, robot)| {
                let occupied = self.occupied_except(index);
                match robot.robot_type {
                    RobotType::Scout => robot.scout_next_pos(&self.map, &occupied),
                    RobotType::Collector => robot.collector_next_pos(&self.map, &occupied),
                }
            })
            .collect()
    }

    fn apply_moves(&mut self, proposals: Vec<Pos>) {
        let current_positions: HashSet<Pos> = self.robots.iter().map(|robot| robot.pos).collect();
        let mut target_counts: HashMap<Pos, usize> = HashMap::new();

        for proposal in &proposals {
            *target_counts.entry(*proposal).or_default() += 1;
        }

        for (index, proposal) in proposals.into_iter().enumerate() {
            let current = self.robots[index].pos;

            if proposal == current {
                continue;
            }
            if !self.map.is_walkable_pos(proposal) {
                continue;
            }
            if target_counts.get(&proposal).copied().unwrap_or(0) > 1 {
                continue;
            }
            if current_positions.contains(&proposal) {
                continue;
            }

            self.robots[index].pos = proposal;
        }
    }

    fn occupied_except(&self, excluded_index: usize) -> HashSet<Pos> {
        self.robots
            .iter()
            .enumerate()
            .filter_map(|(index, robot)| (index != excluded_index).then_some(robot.pos))
            .collect()
    }

    fn remove_depleted_discoveries(&mut self) {
        let depleted: Vec<Pos> = self
            .base
            .discovered_resources
            .keys()
            .copied()
            .filter(|pos| {
                self.map
                    .get(*pos)
                    .is_none_or(|cell| cell.resource_kind().is_none())
            })
            .collect();

        for pos in depleted {
            self.base.forget_resource(pos);
        }
    }
}

fn spawn_positions(base_pos: Pos) -> [Pos; 4] {
    [
        Pos {
            x: base_pos.x.saturating_sub(1),
            y: base_pos.y,
        },
        Pos {
            x: base_pos.x + 1,
            y: base_pos.y,
        },
        Pos {
            x: base_pos.x,
            y: base_pos.y.saturating_sub(1),
        },
        Pos {
            x: base_pos.x,
            y: base_pos.y + 1,
        },
    ]
}
