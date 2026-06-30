# Development Plan — Resource Collection Simulation

## Dependency Graph

```
[Map Generation]        ──────────────────────────────────────┐
[Scout Behavior]        (uses existing Cell/Pos types) ────────┤──► [Integration & main.rs wiring]
[Collector Behavior]    (uses existing Cell/Pos types) ────────┤
[UI Robot Rendering]    (uses robot_positions field)   ────────┘
```

All four Phase 1 tasks can be built against the already-defined stable types
(`Cell`, `Pos`, `Map::get`, `SharedWorld`, `RobotMessage`).
They only need to merge and integrate in Phase 2.

---

## Pre-work — Agree on the shared `Robot` struct (5 min, everyone)

Before splitting, settle the final shape of `Robot` in `robot.rs` so all developers
reference the same type:

```rust
pub struct Robot {
    pub id: usize,
    pub kind: RobotKind,
    pub pos: Pos,
    pub known_cells: HashSet<Pos>,
    pub tx: tokio::sync::mpsc::Sender<RobotMessage>, // added
    pub carrying: Option<ResourceKind>,              // added (collectors)
}
```

Also add one field to `SharedWorld` in `world.rs`:

```rust
pub robot_kinds: HashMap<usize, RobotKind>,
```

---

## Phase 1 — Parallel (4 developers)

### Dev 1 — Map Generation (`map.rs`)

**Fully independent**, no dependency on other Phase 1 work.

1. Replace `Map::empty()` with `Map::generate(width, height, seed)`:
   - Use the `noise` crate (Perlin/simplex) to place `Cell::Obstacle` cells.
2. After obstacle generation, scatter resources on non-obstacle, non-base cells:
   - `Cell::Resource(ResourceKind::Energy, qty)` and `Cell::Resource(ResourceKind::Crystal, qty)`
   - `qty` in range **50–200** using `rand`.
3. Add a public `Map::set(pos: Pos, cell: Cell)` method (needed by collector to
   decrement/remove depleted resources).
4. Update `main.rs` to call `Map::generate(...)` instead of `Map::empty()`.

---

### Dev 2 — Scout Behavior (`robot.rs`, scout portion)

Works against the already-defined `Cell`, `Pos`, `Map::get`, `RobotMessage`.
Can be tested against the flat map.

1. Implement `Robot::step_scout(&mut self, world: &SharedWorld)`:
   - Pick a random adjacent cell (N/S/E/W).
   - If not an obstacle and not out of bounds → move.
   - Scan the 8 neighbors: for any `Cell` not yet in `self.known_cells`,
     send `RobotMessage::Discovered { pos, cell }` and insert into `self.known_cells`.
2. Implement the async task loop:
   ```rust
   pub async fn run_scout(mut robot: Robot, world: Arc<RwLock<SharedWorld>>)
   ```
   - Loop: `step_scout` → update `robot_positions` in world → `sleep(200ms)`.

---

### Dev 3 — Collector Behavior (`robot.rs`, collector portion)

Works against the same existing types. Can be a separate `impl` block.

1. Implement a BFS pathfinding helper:
   ```rust
   fn next_step_toward(from: Pos, to: Pos, world: &SharedWorld) -> Option<Pos>
   ```
   Avoids obstacles using `world.known_cells`.
2. Implement `Robot::step_collector(&mut self, world: &mut SharedWorld)`:
   - **If not carrying**: find the nearest known resource in `world.known_cells`;
     move one step toward it; if adjacent, collect one unit (send
     `RobotMessage::Collected`, set `self.carrying`).
   - **If carrying**: move one step toward `world.map.base_pos()`;
     if at base, unload (`self.carrying = None`).
3. Implement the async task loop:
   ```rust
   pub async fn run_collector(mut robot: Robot, world: Arc<RwLock<SharedWorld>>)
   ```
   - Same pattern as Dev 2.

---

### Dev 4 — UI Robot Rendering (`ui.rs` + minor `world.rs`)

Independent from Devs 1–3 since `robot_positions` and `robot_kinds` are already
in `SharedWorld`.

1. In `render()`, after building map lines, overlay robot positions:
   - Scout → `'x'`, `Color::Red`
   - Collector → `'o'`, `Color::Magenta`
   - If a robot is on a cell, its glyph takes priority over the cell glyph.
2. Improve the status bar:
   - Show robot count, resources remaining, and collected totals.

---

## Parallel Summary

| Developer | File(s)                        | Blocks on                              |
|-----------|--------------------------------|----------------------------------------|
| Dev 1     | `map.rs`                       | Nothing                                |
| Dev 2     | `robot.rs` (scout)             | Nothing (uses existing types)          |
| Dev 3     | `robot.rs` (collector)         | Nothing (uses existing types)          |
| Dev 4     | `ui.rs`, `world.rs` (1 field)  | Pre-work struct agreement              |
| All       | `main.rs`, `base.rs`           | All Phase 1 branches complete          |

---

## Phase 2 — Integration (`main.rs` + `base.rs`)

Sequential, done after all Phase 1 branches are merged.

1. **Spawn robots**: Create N scouts and M collectors (e.g. 3 scouts, 2 collectors),
   all starting at `base_pos`. Register them in `world.robot_positions` and
   `world.robot_kinds`.
2. **Wire channels**: Clone `tx` for each robot and pass it into
   `run_scout` / `run_collector`.
3. **Spawn tasks**: `tokio::spawn(run_scout(...))` / `tokio::spawn(run_collector(...))`
   for each robot.
4. **Resource depletion**: In `base::run`, when a `Collected` message reduces a
   resource to 0, call `world.map.set(pos, Cell::Empty)` and remove from
   `world.known_cells` so collectors stop targeting it.
5. **Edge cases**:
   - Map fully depleted → collectors idle gracefully.
   - Two robots on the same cell → render both or prefer one (no crash).
