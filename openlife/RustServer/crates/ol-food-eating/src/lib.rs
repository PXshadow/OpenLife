//! Pure food-fill + food-store-max helpers (Haxe doEating / calculateFoodStoreMax).
//! Live `try_eat_held` stays in ol-sim.
#![forbid(unsafe_code)]
mod food_fill;
pub mod food_store_max;
pub use food_fill::*;
pub use food_store_max::*;
