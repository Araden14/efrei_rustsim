# Resource Collection Simulation — Architecture Spec

## Entities

| Entity    | Symbol | Owns                                      |
|-----------|--------|-------------------------------------------|
| Map       | —      | tiles (Cell grid), width, height          |
| Base      | `#`    | known_resources, energy_total, crystal_total |
| Scout     | `x`    | pos (local only)                          |
| Collector | `o`    | pos (local only), carrying: Option<ResourceKind> |

## Shared State

Two `Arc<RwLock<_>>` shared across all tasks:

- **Map** — read by everyone, written by collectors (deplete resources)
- **SharedWorld** — wraps Map + Base fields; the single lock robots acquire

## Cell Types

```
Empty | Obstacle | Resource(kind, qty) | Base
```

Resources have qty 50–200. Depleted resources become `Empty`.

## Robot Structs

```rust
pub struct Scout {
    pub pos: Pos,
}

pub struct Collector {
    pub pos: Pos,
    pub carrying: Option<ResourceKind>,
    pub target: Option<Pos>,
}
```

## Robot Vision

- **Scout**: sees 4 adjacent cells (N/S/E/W cross) — discovers resources and obstacles
- **Collector**: no discovery vision; reads known_resources from SharedWorld for targeting; reads map locally only to navigate around obstacles

## Task Layout

```
main
├── tokio::spawn(scout_loop)      × N
├── tokio::spawn(collector_loop)  × N
└── run() → UI loop (blocks until keypress, then exits → all tasks die)
```

## Scout Loop (one tick)

1. Read 4 neighbors from Map (read lock, brief)
2. Pick a walkable neighbor (not Obstacle), move there
3. If neighbor is a Resource → write pos+kind to SharedWorld.known_resources
4. Sleep (tick rate)

## Collector Loop (one tick)

1. If not carrying and no target → read SharedWorld.known_resources, pick nearest
2. Move one step toward target (read Map to avoid obstacles)
3. If at target → collect 1 unit (write lock: decrement qty, or set Empty if depleted)
4. If carrying → navigate back to Base pos
5. If at Base → write lock: increment energy/crystal total, clear carrying
6. Sleep (tick rate)

## Communication

No message channels. Robots communicate through SharedWorld:
- Scout writes to `known_resources`
- Collector reads `known_resources`, writes to map tiles and totals
- UI only reads (never writes)

## UI Loop

- Reads SharedWorld (read lock) every 100ms
- Draws: map tiles, robot positions, resource counters
- Any keypress → returns → process exits

## Startup Setup Screen

Before the sim launches, a plain terminal prompt asks the user:

```
How many scouts?     > _
How many collectors? > _
```

- Read via stdin (no Ratatui for this phase)
- Validate: at least 1 of each, reasonable max (e.g. 10)
- Then hand off to phase 2: spawn robots + run UI loop

## Main Flow

```
phase 1: setup screen → get scout_count, collector_count
phase 2: generate map → spawn robot tasks → run UI loop
```

## What's Not Specced (decide later)

- Tick rate (start with 200ms robots, 100ms UI)
- Pathfinding algorithm (BFS is simplest)
- Collision between robots (ignore for now)
