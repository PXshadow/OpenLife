//! Server → client wire tags (inbound to the client).
//!
//! C++: `protocol.txt` type list; LivingLifePage handlers.
//! Haxe: `openlife/client/ClientTag.hx` (same short codes).
//! RustServer: `ol-protocol::tags::ServerTag` (subset).

/// Server→client message type tag (first line of a framed body).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerTag {
    /// COMPRESSED_MESSAGE
    Cm,
    /// MAP_CHUNK
    Mc,
    /// PLAYER_UPDATE
    Pu,
    /// PLAYER_MOVES_START
    Pm,
    /// PLAYER_OUT_OF_RANGE
    Po,
    /// BABY_WIGGLE
    Bw,
    /// PLAYER_SAYS
    Ps,
    /// LOCATION_SAYS
    Ls,
    /// PLAYER_EMOT
    Pe,
    /// MAP_CHANGE
    Mx,
    /// FOOD_CHANGE
    Fx,
    /// HEAT_CHANGE
    Hx,
    /// LINEAGE
    Ln,
    /// NAME
    Nm,
    /// APOCALYPSE
    Ap,
    /// APOCALYPSE_DONE
    Ad,
    /// DYING
    Dy,
    /// HEALED
    He,
    /// POSSE_JOIN
    Pj,
    /// MONUMENT_CALL
    Mn,
    /// GRAVE
    Gv,
    /// GRAVE_MOVE
    Gm,
    /// GRAVE_OLD
    Go,
    /// STATUE_INFO
    St,
    /// OWNER_LIST
    Ow,
    /// FOLLOWING
    Fw,
    /// EXILED
    Ex,
    /// VALLEY_SPACING
    Vs,
    /// CURSED
    Cu,
    /// CURSE_TOKEN_CHANGE
    Cx,
    /// CURSE_SCORE_CHANGE
    Cs,
    /// FLIGHT_DEST
    Fd,
    /// BAD_BIOMES
    Bb,
    /// VOG_UPDATE
    Vu,
    /// PHOTO_SIGNATURE
    Ph,
    /// FORCED_SHUTDOWN
    Sd,
    /// GLOBAL_MESSAGE
    Ms,
    /// WAR_REPORT
    Wr,
    /// LEARNED_TOOL_REPORT
    Lr,
    /// TOOL_EXPERTS
    Te,
    /// TOOL_SLOTS
    Ts,
    /// HOMELAND
    Hl,
    /// FLIP
    Fl,
    /// CRAVING
    Cr,
    /// FRAME
    Fm,
    /// GHOST
    Gh,
    /// ROCKET_RIDE
    Rr,
    /// ROCKET_ACCOUNT
    Ra,
    /// PONG
    Pong,
    /// SERVER_INFO (pre-login)
    Sn,
    /// ACCEPTED
    Accepted,
    /// REJECTED
    Rejected,
    /// NO_LIFE_TOKENS
    NoLifeTokens,
    /// SHUTDOWN
    Shutdown,
    /// SERVER_FULL
    ServerFull,
    /// Open Life extension (unofficial)
    Ufol,
}

impl ServerTag {
    /// Wire short code (`"MX"`, `"PU"`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cm => "CM",
            Self::Mc => "MC",
            Self::Pu => "PU",
            Self::Pm => "PM",
            Self::Po => "PO",
            Self::Bw => "BW",
            Self::Ps => "PS",
            Self::Ls => "LS",
            Self::Pe => "PE",
            Self::Mx => "MX",
            Self::Fx => "FX",
            Self::Hx => "HX",
            Self::Ln => "LN",
            Self::Nm => "NM",
            Self::Ap => "AP",
            Self::Ad => "AD",
            Self::Dy => "DY",
            Self::He => "HE",
            Self::Pj => "PJ",
            Self::Mn => "MN",
            Self::Gv => "GV",
            Self::Gm => "GM",
            Self::Go => "GO",
            Self::St => "ST",
            Self::Ow => "OW",
            Self::Fw => "FW",
            Self::Ex => "EX",
            Self::Vs => "VS",
            Self::Cu => "CU",
            Self::Cx => "CX",
            Self::Cs => "CS",
            Self::Fd => "FD",
            Self::Bb => "BB",
            Self::Vu => "VU",
            Self::Ph => "PH",
            Self::Sd => "SD",
            Self::Ms => "MS",
            Self::Wr => "WR",
            Self::Lr => "LR",
            Self::Te => "TE",
            Self::Ts => "TS",
            Self::Hl => "HL",
            Self::Fl => "FL",
            Self::Cr => "CR",
            Self::Fm => "FM",
            Self::Gh => "GH",
            Self::Rr => "RR",
            Self::Ra => "RA",
            Self::Pong => "PONG",
            Self::Sn => "SN",
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::NoLifeTokens => "NO_LIFE_TOKENS",
            Self::Shutdown => "SHUTDOWN",
            Self::ServerFull => "SERVER_FULL",
            Self::Ufol => "UFOL",
        }
    }

    /// Parse first-line tag token (case-sensitive, as on the wire).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "CM" => Self::Cm,
            "MC" => Self::Mc,
            "PU" => Self::Pu,
            "PM" => Self::Pm,
            "PO" => Self::Po,
            "BW" => Self::Bw,
            "PS" => Self::Ps,
            "LS" => Self::Ls,
            "PE" => Self::Pe,
            "MX" => Self::Mx,
            "FX" => Self::Fx,
            "HX" => Self::Hx,
            "LN" => Self::Ln,
            "NM" => Self::Nm,
            "AP" => Self::Ap,
            "AD" => Self::Ad,
            "DY" => Self::Dy,
            "HE" => Self::He,
            "PJ" => Self::Pj,
            "MN" => Self::Mn,
            "GV" => Self::Gv,
            "GM" => Self::Gm,
            "GO" => Self::Go,
            "ST" => Self::St,
            "OW" => Self::Ow,
            "FW" => Self::Fw,
            "EX" => Self::Ex,
            "VS" => Self::Vs,
            "CU" => Self::Cu,
            "CX" => Self::Cx,
            "CS" => Self::Cs,
            "FD" => Self::Fd,
            "BB" => Self::Bb,
            "VU" => Self::Vu,
            "PH" => Self::Ph,
            "SD" => Self::Sd,
            "MS" => Self::Ms,
            "WR" => Self::Wr,
            "LR" => Self::Lr,
            "TE" => Self::Te,
            "TS" => Self::Ts,
            "HL" => Self::Hl,
            "FL" => Self::Fl,
            "CR" => Self::Cr,
            "FM" => Self::Fm,
            "GH" => Self::Gh,
            "RR" => Self::Rr,
            "RA" => Self::Ra,
            "PONG" => Self::Pong,
            "SN" => Self::Sn,
            "ACCEPTED" => Self::Accepted,
            "REJECTED" => Self::Rejected,
            "NO_LIFE_TOKENS" => Self::NoLifeTokens,
            "SHUTDOWN" => Self::Shutdown,
            "SERVER_FULL" => Self::ServerFull,
            "UFOL" => Self::Ufol,
            _ => return None,
        })
    }

    /// True if this tag may carry a zlib binary payload after `#` (frame reader).
    pub fn has_binary_after_hash(self) -> bool {
        matches!(self, Self::Mc | Self::Cm)
    }

    /// Tags that are commonly needed to stay in-world without desync (P0 parse surface).
    pub fn is_live_critical(self) -> bool {
        matches!(
            self,
            Self::Mc
                | Self::Pu
                | Self::Pm
                | Self::Po
                | Self::Mx
                | Self::Fx
                | Self::Hx
                | Self::Fm
                | Self::Ps
                | Self::Pe
                | Self::Bw
                | Self::Ls
                | Self::Nm
                | Self::Ln
                | Self::Dy
                | Self::He
                | Self::Sd
                | Self::Ms
        )
    }
}

/// All known wire codes (for exhaustive tests / docs).
pub const ALL_SERVER_TAGS: &[ServerTag] = &[
    ServerTag::Cm,
    ServerTag::Mc,
    ServerTag::Pu,
    ServerTag::Pm,
    ServerTag::Po,
    ServerTag::Bw,
    ServerTag::Ps,
    ServerTag::Ls,
    ServerTag::Pe,
    ServerTag::Mx,
    ServerTag::Fx,
    ServerTag::Hx,
    ServerTag::Ln,
    ServerTag::Nm,
    ServerTag::Ap,
    ServerTag::Ad,
    ServerTag::Dy,
    ServerTag::He,
    ServerTag::Pj,
    ServerTag::Mn,
    ServerTag::Gv,
    ServerTag::Gm,
    ServerTag::Go,
    ServerTag::St,
    ServerTag::Ow,
    ServerTag::Fw,
    ServerTag::Ex,
    ServerTag::Vs,
    ServerTag::Cu,
    ServerTag::Cx,
    ServerTag::Cs,
    ServerTag::Fd,
    ServerTag::Bb,
    ServerTag::Vu,
    ServerTag::Ph,
    ServerTag::Sd,
    ServerTag::Ms,
    ServerTag::Wr,
    ServerTag::Lr,
    ServerTag::Te,
    ServerTag::Ts,
    ServerTag::Hl,
    ServerTag::Fl,
    ServerTag::Cr,
    ServerTag::Fm,
    ServerTag::Gh,
    ServerTag::Rr,
    ServerTag::Ra,
    ServerTag::Pong,
    ServerTag::Sn,
    ServerTag::Accepted,
    ServerTag::Rejected,
    ServerTag::NoLifeTokens,
    ServerTag::Shutdown,
    ServerTag::ServerFull,
    ServerTag::Ufol,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tags_roundtrip() {
        for &t in ALL_SERVER_TAGS {
            assert_eq!(ServerTag::parse(t.as_str()), Some(t), "{}", t.as_str());
        }
    }

    #[test]
    fn unknown_tag_none() {
        assert_eq!(ServerTag::parse("ZZ"), None);
        assert_eq!(ServerTag::parse("mx"), None); // wire is uppercase
    }

    #[test]
    fn binary_tags() {
        assert!(ServerTag::Mc.has_binary_after_hash());
        assert!(ServerTag::Cm.has_binary_after_hash());
        assert!(!ServerTag::Mx.has_binary_after_hash());
    }
}
