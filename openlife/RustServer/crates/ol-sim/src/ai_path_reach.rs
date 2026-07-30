//! Live AI path-block maps — re-exported from **`ol-ai-pathing`** (AI crate split).
//!
//! Pure timers / fail-marks / sticky food / `CalculateBlockedByAi` live in
//! `ol-ai-pathing`. Player fields and live rebuild stay in `ol-sim`.
//!
//! Prefer editing `crates/ol-ai-pathing/src/ai_path_reach.rs`. Historical
//! `build_*.rs` patches that target this path should become no-ops once the
//! body is only a re-export.

pub use ol_ai_pathing::ai_path_reach::*;
