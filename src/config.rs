use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use std::path::Path;

/// Top-level configuration loaded from `config.toml`.
/// Every field has a `Default` impl so the file is entirely optional —
/// missing keys fall back to the built-in defaults.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub simulation: SimulationConfig,
    pub robots: RobotsConfig,
    pub map: MapConfig,
}

/// Simulation-level knobs (robot counts, UI framerate).
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SimulationConfig {
    /// Number of scout robots spawned at startup.
    pub num_scouts: usize,
    /// Number of collector robots spawned at startup.
    pub num_collectors: usize,
    /// UI redraw / event-poll interval in milliseconds.
    pub ui_poll_ms: u64,
}

/// Per-robot timing configuration.
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RobotsConfig {
    /// How often each scout moves and scans, in milliseconds.
    pub scout_tick_ms: u64,
    /// How often each collector acts, in milliseconds.
    pub collector_tick_ms: u64,
}

/// Map-generation parameters.
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MapConfig {
    /// Number of deposits placed per resource type (Energy and Crystal each).
    pub resources_per_kind: usize,
    /// Perlin-noise cutoff for obstacles: cells whose noise value exceeds this
    /// become `Cell::Obstacle`. Range 0.0–1.0; higher = fewer obstacles.
    pub obstacle_threshold: f64,
    /// Noise sampling frequency. Lower = larger smooth blobs; higher = more
    /// fragmented terrain.
    pub noise_scale: f64,
    /// Minimum number of units in a resource deposit.
    pub resource_qty_min: u32,
    /// Maximum number of units in a resource deposit.
    pub resource_qty_max: u32,
}

// ── Default values ──────────────────────────────────────────────────────────

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            num_scouts: 10,
            num_collectors: 5,
            ui_poll_ms: 50,
        }
    }
}

impl Default for RobotsConfig {
    fn default() -> Self {
        Self {
            scout_tick_ms: 200,
            collector_tick_ms: 200,
        }
    }
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            resources_per_kind: 10,
            obstacle_threshold: 0.25,
            noise_scale: 0.12,
            resource_qty_min: 50,
            resource_qty_max: 200,
        }
    }
}

// ── Loading ──────────────────────────────────────────────────────────────────

impl Config {
    /// Load `config.toml` from the current working directory.
    /// Falls back to defaults when the file does not exist.
    /// Returns an error when the file exists but cannot be parsed.
    pub fn load() -> color_eyre::Result<Self> {
        let path = Path::new("config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path).wrap_err("failed to read config.toml")?;
        toml::from_str(&content).wrap_err("failed to parse config.toml")
    }
}
