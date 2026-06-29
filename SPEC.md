# Resource Collection Simulation — Architecture Spec

## Entities

| Entity    | Symbol | Color      | Owns                                              |
|-----------|--------|------------|---------------------------------------------------|
| Map       | —      | —          | tiles (Cell grid), width, height                  |
| Base      | `#`    | LightGreen | known_resources, energy_total, crystal_total      |
| Scout     | `x`    | White      | pos (local to task), id                           |
| Collector | `o`    | LightBlue  | pos (local to task), id, carrying, target         |
| Energy    | `E`    | Yellow     | qty 50–200                                        |
| Crystal   | `C`    | LightMagenta | qty 50–200                                      |
| Obstacle  | `O`    | LightCyan  | —                                                 |

## Shared State

One `Arc<RwLock<SharedWorld>>` shared across all tasks:

```rust
pub struct SharedWorld {
    pub map: Map,                              // tile grid
    pub base_pos: Pos,                         // center of map
    pub known_resources: Vec<(Pos, ResourceKind)>,
    pub scout_positions: Vec<Pos>,             // indexed by scout id
    pub collector_positions: Vec<Pos>,         // indexed by collector id
    pub energy_collected: u32,
    pub crystal_collected: u32,
}
```

## Cell Types

```
Empty | Obstacle | Resource(kind, qty) | Base
```

Resources have qty 50–200. Depleted resources become `Empty`.

## Robot Task Signatures

```rust
pub async fn scout_loop(world: Arc<RwLock<SharedWorld>>, start: Pos, id: usize)
pub async fn collector_loop(world: Arc<RwLock<SharedWorld>>, start: Pos, id: usize)
```

State local to each task (not in SharedWorld): `pos`, `carrying`, `target`.

## Robot Vision

- **Scout**: sees 4 adjacent cells (N/S/E/W cross) — discovers resources
- **Collector**: no discovery vision; reads `known_resources` from SharedWorld; uses BFS to navigate

## Task Layout

```
main
├── tokio::spawn(scout_loop(world, base_pos, id))      × N
├── tokio::spawn(collector_loop(world, base_pos, id))  × N
└── run() → UI loop (blocks until keypress, then exits → all tasks die)
```

## Scout Loop (one tick, 200ms)

1. Read lock: get 4 neighbors, pick a walkable one (not Obstacle) at random
2. Write lock: update `scout_positions[id]`; if new cell is Resource, append to `known_resources` if not already present
3. Sleep 200ms

## Collector Loop (one tick, 200ms)

1. Read lock: if idle (no target, not carrying) → pick nearest known resource by Manhattan distance; BFS one step toward target or base
2. Move to next step
3. Write lock: update `collector_positions[id]`; on arrival at resource → collect 1 unit (decrement qty, set Empty if 0), remove from `known_resources`, set carrying; on arrival at base → increment counter, clear carrying
4. If resource was gone on arrival → clear target, go idle
5. Sleep 200ms

## Pathfinding

BFS from current pos to destination, returning only the next step. Obstacles are impassable; all other cells (Empty, Resource, Base) are walkable. Full BFS runs each tick — acceptable for map sizes in use.

## Communication

No message channels. All coordination via SharedWorld under RwLock:
- Scout writes to `known_resources` and `scout_positions`
- Collector reads `known_resources`, writes to map tiles, totals, and `collector_positions`
- UI only reads (never writes)

## UI Loop (100ms)

- Read lock every 100ms, render full frame
- Robot glyphs overlay cell glyphs (robot takes priority at same position)
- Status bar: `energy: N  crystal: N  (any key to quit)`

## Startup Setup Screen

```
How many scouts?     > _
How many collectors? > _
```

- Plain stdin, no Ratatui
- Validates 1–10 for each
- Then: generate map → pre-fill position vecs → spawn tasks → run UI loop

## Main Flow

```
phase 1: setup()  → scout_count, collector_count
phase 2: generate map → Arc<RwLock<SharedWorld>>
         pre-fill scout_positions / collector_positions with base_pos
         spawn N scout tasks, M collector tasks
         run UI loop until keypress
```

## What's Not Specced (decide later)

- Collision between robots (currently ignored — multiple robots can share a cell)
- Resource collection amount (currently 1 unit per trip; whole-node removal is a trivial change)
