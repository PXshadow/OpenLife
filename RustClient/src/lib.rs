//! Headless One Hour One Life client library.
//!
//! Wire format follows Jason Rohrer's `server/protocol.txt` and the official
//! client's `LivingLifePage.cpp` send paths (LOGIN/RLOGIN, MOVE, USE/DROP/REMV/SELF/KA/FORCE).

pub mod actions;
pub mod frame;
pub mod login;
pub mod move_state;
pub mod parse;
pub mod session;
pub mod wire_log;

pub use actions::{
    ObjectAction, encode_baby, encode_drop, encode_force, encode_ka, encode_remv, encode_say,
    encode_self, encode_sremv, encode_swap, encode_ubaby, encode_use,
};
pub use frame::{FrameReader, encode_raw};
pub use login::{
    LoginParams, encode_login, hmac_sha1_hex, pure_account_key,
};
pub use move_state::{MAX_PATH_DELTA, MoveError, MoveState, PathDelta, encode_move};
pub use parse::{
    LoginOutcome, PlayerMoveStart, PlayerUpdate, ServerHello, parse_login_outcome, parse_pm_line,
    parse_pm_message, parse_pu_line, parse_sn,
};
pub use session::{
    SessionConfig, SessionEvent, connect_and_login, connect_and_login_logged,
};
pub use wire_log::WireLog;
