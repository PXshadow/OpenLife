//! Pure heat / comfort helpers (Haxe updateTemperature math).
//! Live tile temps + Player.heat apply stay in ol-sim temperature_handler.
#![forbid(unsafe_code)]
mod heat_ideal;
pub use heat_ideal::*;
