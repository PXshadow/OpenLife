//! Read-only view of one living body (part of [`crate::PlayerReadInterface`]).

/// Snapshot-style player sensors. Adapters fill from `Player` / `PlayerSnapshot`.
pub trait PlayerView {
    fn conn_id(&self) -> u64;
    fn p_id(&self) -> i32;
    fn pos(&self) -> (i32, i32);
    fn age(&self) -> f32;
    /// `(food_store, food_max)`.
    fn food(&self) -> (f32, f32);
    fn held_id(&self) -> i32;
    fn home(&self) -> (i32, i32);
    /// Six clothing parent ids (Haxe clothingObjects).
    fn clothing(&self) -> [i32; 6];
    fn deleted(&self) -> bool;
    fn is_moving(&self) -> bool;
}
