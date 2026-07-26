//! Server configuration loaded from TOML.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub game_port: u16,
    pub web_port: u16,
    pub bind: String,
    pub max_players: u32,
    pub required_version: i32,
    /// When true, LOGIN with a numeric `client_tag` that does not match
    /// [`Self::required_version`] is hard-rejected (PS reject + no spawn).
    /// Default **false** (soft-log only). Normal OHOL tags like `client_official`
    /// are not numeric and are never treated as a version mismatch.
    pub client_version_strict: bool,
    pub content_path: PathBuf,
    pub challenge_len: usize,
    pub tick_hz: u32,
    /// Time dilation: multiplies `dt` passed into sim vitals (`1.0` = realtime).
    /// Values `> 1` speed up aging/food/etc.; `0` freezes vitals time (same as pause).
    pub sim_speed: f32,
    pub enable_game_net: bool,
    pub enable_web: bool,

    /// Mirror Haxe ticket-server account check on LOGIN. Default **on**.
    pub verify_ohol_ticket: bool,
    /// Ticket endpoint (Haxe default host path).
    pub ticket_verify_url: String,

    /// PNG biome map (Haxe `MapFileName`).
    pub map_png_path: PathBuf,
    /// Save directory for versioned binary world (Haxe `SaveDirectory`).
    pub save_directory: PathBuf,
    /// If true and no save exists, generate from PNG + natural objects.
    pub generate_map_if_missing: bool,
    /// Force regenerate even if save exists.
    pub force_regenerate_map: bool,
    /// Density factor for natural object placement (Haxe ~0.4 gate).
    pub natural_object_density: f32,

    /// Spawn in-process self-play agents (dev / viewer). Default **on**.
    pub selfplay_enabled: bool,
    /// Number of self-play agents to spawn (clamped to 1–3): Forager, +Farmer, +Hunter.
    pub selfplay_agents: u8,
    /// Cap on transitions seeded into the reverse craft graph at boot (fast restart).
    pub craft_graph_seed_cap: usize,

    /// Multi-server twin peer endpoints (**stub only** — listed in sim, no network I/O).
    ///
    /// Empty by default. When non-empty, peers appear in `SAY ?TWINS` after boot seed.
    pub twin_peers: Vec<TwinPeerConfig>,

    /// Timed multi-tile MovePath + PM (Haxe MoveHelper). Default **on** (Haxe-like; not instant).
    pub timed_movement: bool,
    /// AI craft search radius (tiles) for bottom-up valuation.
    pub ai_craft_radius: i32,
    /// Instant MOVE only: max Chebyshev snap of start tile (default 2).
    /// Timed MovePath uses Haxe `MaxMovementQuadJumpDistanceBeforeForce` (quadDist ≤ 5)
    /// and ignores this field — do not raise it to “widen” timed jumps.
    pub move_jump_max_chebyshev: i32,
    /// Max intents applied per tick wake (fairness under self-play/AI flood).
    pub intent_drain_budget: u32,
    /// Ops series sample every N ticks (100 @ 20 Hz ≈ 5 s).
    pub ops_sample_every_ticks: u64,
    /// Flush ops journal every N seconds (default 300).
    pub ops_flush_secs: u64,
    /// Path for ops metrics journal under SaveFiles.
    pub ops_journal_path: PathBuf,
    /// AI NPC scheduler (default **on** — floor `npc_min` agents when enabled).
    pub npc_enabled: bool,
    /// When `npc_enabled`, floor population (Forager/Farmer/Hunter-style).
    pub npc_min: u32,
    /// Adaptive AI population ceiling.
    pub npc_max: u32,
    /// Each NPC thinks every N ticks (stagger by p_id).
    pub ai_think_period_ticks: u32,
    /// Observation radius (tiles) for AI brain snapshot.
    pub ai_observe_radius: i32,
    /// When true, MX/PU fan-out to all connected clients (ignore distance).
    pub broadcast_all_updates: bool,
    /// `SAY !shutdown` global countdown seconds before save + apocalypse (default 3).
    pub shutdown_countdown_secs: u32,
    /// Seconds to display apocalypse signal after save before orderly exit (default 3).
    pub shutdown_apocalypse_secs: u32,
}

/// One configured twin peer host:port (no last_pong — that lives in the sim stub registry).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TwinPeerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            game_port: 8005,
            web_port: 8080,
            bind: "0.0.0.0".into(),
            max_players: 200,
            required_version: 437,
            client_version_strict: false,
            content_path: PathBuf::from("content/OneLifeData7"),
            challenge_len: 48,
            tick_hz: 20,
            sim_speed: 1.0,
            enable_game_net: true,
            enable_web: true,
            verify_ohol_ticket: true,
            ticket_verify_url: "https://onehouronelife.com/ticketServer/server.php".into(),
            map_png_path: PathBuf::from("maps/mysteraV1Test.png"),
            save_directory: PathBuf::from("SaveFiles"),
            generate_map_if_missing: true,
            force_regenerate_map: false,
            natural_object_density: 0.4,
            selfplay_enabled: true,
            selfplay_agents: 3,
            craft_graph_seed_cap: 50_000,
            twin_peers: Vec::new(),
            timed_movement: true,
            ai_craft_radius: 50,
            // Instant MOVE snap only. Timed path uses Haxe quadDist ≤ 5 (not this field).
            move_jump_max_chebyshev: 2,
            intent_drain_budget: 64,
            ops_sample_every_ticks: 100,
            ops_flush_secs: 300,
            ops_journal_path: PathBuf::from("SaveFiles/ops_metrics.journal"),
            npc_enabled: true,
            npc_min: 3,
            npc_max: 40,
            ai_think_period_ticks: 10,
            ai_observe_radius: 16,
            broadcast_all_updates: true,
            shutdown_countdown_secs: 3,
            shutdown_apocalypse_secs: 3,
        }
    }
}

impl ServerConfig {
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        let cfg: ServerConfig = toml::from_str(&text)?;
        Ok(cfg)
    }

    pub fn write_default(path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(&Self::default()).expect("serialize default config");
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text)?;
        Ok(())
    }

    pub fn game_addr(&self) -> String {
        format!("{}:{}", self.bind, self.game_port)
    }

    pub fn web_addr(&self) -> String {
        format!("{}:{}", self.bind, self.web_port)
    }

    pub fn world_save_path(&self) -> PathBuf {
        self.save_directory.join("world_v1.olw")
    }

    /// Versioned binary lineage index (`OLN1` / `lineages_v1.bin`).
    pub fn lineage_save_path(&self) -> PathBuf {
        self.save_directory.join("lineages_v1.bin")
    }

    /// Versioned binary soft-account book (`OLA1` / `accounts_v1.bin`).
    pub fn accounts_save_path(&self) -> PathBuf {
        self.save_directory.join("accounts_v1.bin")
    }

    /// Self-play agent count clamped to **1..=3**.
    pub fn selfplay_agent_count(&self) -> u8 {
        self.selfplay_agents.clamp(1, 3)
    }

    /// Craft-graph seed cap (at least 1 so seed never panics on zero).
    pub fn craft_graph_cap(&self) -> usize {
        self.craft_graph_seed_cap.max(1)
    }

    /// Sim time dilation, clamped to non-negative (`0` freezes vitals dt).
    pub fn sim_speed_factor(&self) -> f32 {
        if self.sim_speed.is_finite() && self.sim_speed >= 0.0 {
            self.sim_speed
        } else {
            1.0
        }
    }

    /// Intent drain budget per tick wake (at least 1).
    pub fn intent_drain(&self) -> usize {
        self.intent_drain_budget.max(1) as usize
    }

    /// NPC min/max when enabled (min ≥ 1 when enabled path uses it; config may be 0).
    pub fn npc_bounds(&self) -> (u32, u32) {
        let min = self.npc_min;
        let max = self.npc_max.max(min);
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_default_on() {
        let c = ServerConfig::default();
        assert!(c.verify_ohol_ticket);
    }

    #[test]
    fn client_version_strict_default_off() {
        let c = ServerConfig::default();
        assert!(!c.client_version_strict);
        let back: ServerConfig = toml::from_str("client_version_strict = true").unwrap();
        assert!(back.client_version_strict);
    }

    #[test]
    fn selfplay_and_craft_defaults() {
        let c = ServerConfig::default();
        assert!(c.selfplay_enabled);
        assert_eq!(c.selfplay_agents, 3);
        assert_eq!(c.selfplay_agent_count(), 3);
        assert_eq!(c.craft_graph_seed_cap, 50_000);
        assert_eq!(c.craft_graph_cap(), 50_000);
        let clamped = ServerConfig {
            selfplay_agents: 99,
            craft_graph_seed_cap: 0,
            ..Default::default()
        };
        assert_eq!(clamped.selfplay_agent_count(), 3);
        assert_eq!(clamped.craft_graph_cap(), 1);
    }

    #[test]
    fn roundtrip_toml() {
        let c = ServerConfig {
            verify_ohol_ticket: false,
            selfplay_enabled: false,
            selfplay_agents: 1,
            craft_graph_seed_cap: 1_000,
            twin_peers: vec![TwinPeerConfig {
                host: "127.0.0.1".into(),
                port: 8006,
            }],
            ..Default::default()
        };
        let s = toml::to_string(&c).unwrap();
        let back: ServerConfig = toml::from_str(&s).unwrap();
        assert!(!back.verify_ohol_ticket);
        assert!(!back.selfplay_enabled);
        assert_eq!(back.selfplay_agents, 1);
        assert_eq!(back.craft_graph_seed_cap, 1_000);
        assert_eq!(back.twin_peers.len(), 1);
        assert_eq!(back.twin_peers[0].host, "127.0.0.1");
        assert_eq!(back.twin_peers[0].port, 8006);
    }

    #[test]
    fn twin_peers_default_empty() {
        let c = ServerConfig::default();
        assert!(c.twin_peers.is_empty());
        let back: ServerConfig = toml::from_str("").unwrap();
        assert!(back.twin_peers.is_empty());
    }

    #[test]
    fn movement_and_npc_defaults() {
        let c = ServerConfig::default();
        assert!(c.timed_movement);
        assert_eq!(c.ai_craft_radius, 50);
        assert_eq!(c.move_jump_max_chebyshev, 2);
        assert_eq!(c.intent_drain(), 64);
        assert!(c.npc_enabled);
        assert_eq!(c.npc_min, 3);
        assert_eq!(c.npc_max, 40);
        let on: ServerConfig = toml::from_str("timed_movement = true\nnpc_enabled = true").unwrap();
        assert!(on.timed_movement);
        assert!(on.npc_enabled);
        let off: ServerConfig = toml::from_str("npc_enabled = false").unwrap();
        assert!(!off.npc_enabled);
    }

    #[test]
    fn sim_speed_default_and_clamp() {
        let c = ServerConfig::default();
        assert!((c.sim_speed - 1.0).abs() < f32::EPSILON);
        assert!((c.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let fast = ServerConfig {
            sim_speed: 2.5,
            ..Default::default()
        };
        assert!((fast.sim_speed_factor() - 2.5).abs() < f32::EPSILON);
        let bad = ServerConfig {
            sim_speed: f32::NAN,
            ..Default::default()
        };
        assert!((bad.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let neg = ServerConfig {
            sim_speed: -3.0,
            ..Default::default()
        };
        assert!((neg.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let zero = ServerConfig {
            sim_speed: 0.0,
            ..Default::default()
        };
        assert!((zero.sim_speed_factor() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sim_speed_toml_roundtrip() {
        let c = ServerConfig {
            sim_speed: 4.0,
            ..Default::default()
        };
        let s = toml::to_string(&c).unwrap();
        let back: ServerConfig = toml::from_str(&s).unwrap();
        assert!((back.sim_speed - 4.0).abs() < f32::EPSILON);
    }
}
