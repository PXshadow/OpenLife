//! Toroidal wrap math — **canonical implementation** in `ol-move-rules`.
//!
//! Kept as a thin re-export so existing `crate::math_wrap::…` paths keep working.

pub use ol_move_rules::{
    chebyshev_wrap, euclidean_wrap, format_wrap_query, manhattan_wrap, step_wrap, wrap_axis,
    wrap_delta, wrap_delta_1d, wrap_tile,
};
