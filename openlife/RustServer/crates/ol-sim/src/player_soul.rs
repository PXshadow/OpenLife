//! PlayerSoul — AI context / interaction memory (Haxe `openlife.server.PlayerSoul`).
//!
//! Pure memory FIFO + string builders for LLM roleplay prompts.
//! Not the account/grave "soul" token (`death_inherit::account_soul_token`).
//!
//! Chunks: **S-SOUL** (pure) + **AI-SOUL-WIRE** (`Player.soul` + live SoulView).
//! AiHandler prompt assembly remains a separate gap (S-AIH).

// Pure body (memory + prompt builders + unit tests).
#[path = "player_soul_body.rs"]
mod body;
pub use body::*;

// Live wire helpers (angry/female/season/profession sticky).
#[path = "player_soul_wire.rs"]
mod wire;
pub use wire::*;

// SimState::soul_view_for / player_soul_text / add_player_soul_* (AI-SOUL-WIRE).
// Must load after crate root defines SimState; this module is declared early in lib.rs
// so the impl is attached when the crate finishes compiling.
#[path = "soul_live.rs"]
mod soul_live;
