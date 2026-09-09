//! Pure combat formulas (angryTime, reputation labels, wound plans).
//! Live HIT/KILL / setHeld wound apply stays in ol-sim.
#![forbid(unsafe_code)]
mod angry;
mod bloody;
mod fever;
mod reputation;
mod weapon_wound;
mod wound_desc;
pub use angry::*;
pub use bloody::*;
pub use fever::*;
pub use reputation::*;
pub use weapon_wound::*;
pub use wound_desc::*;
