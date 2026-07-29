//! Known OHOL / Open Life wire tags (client → server and server → client).

/// Client → server command tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientTag {
    Login,
    RLogin,
    Ka,
    Use,
    Drop,
    Remv,
    Sremv,
    Self_,
    Baby,
    Ubaby,
    Swap,
    Kill,
    Jump,
    Emot,
    Die,
    Grave,
    Owner,
    Force,
    Ping,
    Vogs,
    Vogn,
    Vogp,
    Vogm,
    Vogi,
    Vogt,
    Vogx,
    /// Client photo signature request: PHOTO x y seq
    Photo,
    Map,
    Trigger,
    Say,
    Move,
}

impl ClientTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Login => "LOGIN",
            Self::RLogin => "RLOGIN",
            Self::Ka => "KA",
            Self::Use => "USE",
            Self::Drop => "DROP",
            Self::Remv => "REMV",
            Self::Sremv => "SREMV",
            Self::Self_ => "SELF",
            Self::Baby => "BABY",
            Self::Ubaby => "UBABY",
            Self::Swap => "SWAP",
            Self::Kill => "KILL",
            Self::Jump => "JUMP",
            Self::Emot => "EMOT",
            Self::Die => "DIE",
            Self::Grave => "GRAVE",
            Self::Owner => "OWNER",
            Self::Force => "FORCE",
            Self::Ping => "PING",
            Self::Vogs => "VOGS",
            Self::Vogn => "VOGN",
            Self::Vogp => "VOGP",
            Self::Vogm => "VOGM",
            Self::Vogi => "VOGI",
            Self::Vogt => "VOGT",
            Self::Vogx => "VOGX",
            Self::Photo => "PHOTO",
            Self::Map => "MAP",
            Self::Trigger => "TRIGGER",
            Self::Say => "SAY",
            Self::Move => "MOVE",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "LOGIN" => Self::Login,
            "RLOGIN" => Self::RLogin,
            "KA" => Self::Ka,
            "USE" => Self::Use,
            "DROP" => Self::Drop,
            "REMV" => Self::Remv,
            "SREMV" => Self::Sremv,
            "SELF" => Self::Self_,
            "BABY" => Self::Baby,
            "UBABY" => Self::Ubaby,
            "SWAP" => Self::Swap,
            "KILL" => Self::Kill,
            "JUMP" => Self::Jump,
            "EMOT" => Self::Emot,
            "DIE" => Self::Die,
            "GRAVE" => Self::Grave,
            "OWNER" => Self::Owner,
            "FORCE" => Self::Force,
            "PING" => Self::Ping,
            "VOGS" => Self::Vogs,
            "VOGN" => Self::Vogn,
            "VOGP" => Self::Vogp,
            "VOGM" => Self::Vogm,
            "VOGI" => Self::Vogi,
            "VOGT" => Self::Vogt,
            "VOGX" => Self::Vogx,
            "PHOTO" => Self::Photo,
            "MAP" => Self::Map,
            "TRIGGER" => Self::Trigger,
            "SAY" => Self::Say,
            "MOVE" => Self::Move,
            _ => return None,
        })
    }

    /// True for Voice-of-God client commands (VOGS/VOGN/VOGP/VOGM/VOGI/VOGT/VOGX).
    pub fn is_vog(self) -> bool {
        matches!(
            self,
            Self::Vogs | Self::Vogn | Self::Vogp | Self::Vogm | Self::Vogi | Self::Vogt | Self::Vogx
        )
    }
}

/// Server → client message tags (short names used on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerTag {
    Sn,
    Accepted,
    Rejected,
    Shutdown,
    ServerFull,
    NoLifeTokens,
    Pu,
    Pm,
    Po,
    Mc,
    Mx,
    Fx,
    Hx,
    Ps,
    Pe,
    /// BABY_WIGGLE — babies that just started wiggling
    Bw,
    Ln,
    Nm,
    Cm,
    /// DYING — mortally wounded / dying soon (optional isSick flag)
    Dy,
    He,
    Gv,
    /// CURSE_TOKEN_CHANGE
    Cx,
    /// CURSE_SCORE_CHANGE
    Cs,
    /// VOG_UPDATE — teleport / VoG camera position
    Vu,
    /// PHOTO_SIGNATURE — reply to client PHOTO request
    Ph,
    /// LEARNED_TOOL_REPORT
    Lr,
    /// TOOL_SLOTS
    Ts,
    /// TOOL_EXPERTS
    Te,
    /// PONG — reply to client PING (echo unique_id)
    Pong,
    /// CRAVING — food_id bonus after eat (Haxe ClientTag.CRAVING)
    Cr,
    // Keep extending as needed
}

impl ServerTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sn => "SN",
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::Shutdown => "SHUTDOWN",
            Self::ServerFull => "SERVER_FULL",
            Self::NoLifeTokens => "NO_LIFE_TOKENS",
            Self::Pu => "PU",
            Self::Pm => "PM",
            Self::Po => "PO",
            Self::Mc => "MC",
            Self::Mx => "MX",
            Self::Fx => "FX",
            Self::Hx => "HX",
            Self::Ps => "PS",
            Self::Pe => "PE",
            Self::Bw => "BW",
            Self::Ln => "LN",
            Self::Nm => "NM",
            Self::Cm => "CM",
            Self::Dy => "DY",
            Self::He => "HE",
            Self::Gv => "GV",
            Self::Cx => "CX",
            Self::Cs => "CS",
            Self::Vu => "VU",
            Self::Ph => "PH",
            Self::Lr => "LR",
            Self::Ts => "TS",
            Self::Te => "TE",
            Self::Pong => "PONG",
            Self::Cr => "CR",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "SN" => Self::Sn,
            "ACCEPTED" => Self::Accepted,
            "REJECTED" => Self::Rejected,
            "SHUTDOWN" => Self::Shutdown,
            "SERVER_FULL" => Self::ServerFull,
            "NO_LIFE_TOKENS" => Self::NoLifeTokens,
            "PU" => Self::Pu,
            "PM" => Self::Pm,
            "PO" => Self::Po,
            "MC" => Self::Mc,
            "MX" => Self::Mx,
            "FX" => Self::Fx,
            "HX" => Self::Hx,
            "PS" => Self::Ps,
            "PE" => Self::Pe,
            "BW" => Self::Bw,
            "LN" => Self::Ln,
            "NM" => Self::Nm,
            "CM" => Self::Cm,
            "DY" => Self::Dy,
            "HE" => Self::He,
            "GV" => Self::Gv,
            "CX" => Self::Cx,
            "CS" => Self::Cs,
            "VU" => Self::Vu,
            "PH" => Self::Ph,
            "LR" => Self::Lr,
            "TS" => Self::Ts,
            "TE" => Self::Te,
            "PONG" => Self::Pong,
            "CR" => Self::Cr,
            _ => return None,
        })
    }
}

/// Dummy PHOTO_SIGNATURE when photos are denied / unsupported.
pub const PHOTO_DENIED_SIGNATURE: &str = "DENIED";

/// Format VOG_UPDATE (VU) server→client: `x y`.
pub fn format_vog_update(x: i32, y: i32) -> String {
    crate::format_server_message("VU", &[&format!("{x} {y}")])
}

/// Format PHOTO_SIGNATURE (PH) server→client: `x y signature`.
pub fn format_photo_signature(x: i32, y: i32, signature: &str) -> String {
    crate::format_server_message("PH", &[&format!("{x} {y} {signature}")])
}

/// Format PONG server→client: echo of client PING `unique_id` (x/y ignored).
/// Wire: `PONG\nunique_id\n#` (protocol.txt).
pub fn format_pong(unique_id: &str) -> String {
    crate::format_server_message("PONG", &[unique_id])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{format_learned_tool_report, format_tool_slots};

    #[test]
    fn client_tags_roundtrip() {
        for s in ["LOGIN", "USE", "DROP", "KA", "SAY", "KILL"] {
            let t = ClientTag::parse(s).unwrap();
            assert_eq!(t.as_str(), s);
        }
    }

    #[test]
    fn photo_and_vog_client_tags_roundtrip() {
        for s in ["PHOTO", "VOGS", "VOGN", "VOGP", "VOGM", "VOGI", "VOGT", "VOGX"] {
            let t = ClientTag::parse(s).unwrap();
            assert_eq!(t.as_str(), s);
        }
        assert!(ClientTag::Vogs.is_vog());
        assert!(ClientTag::Photo.is_vog() == false);
    }

    #[test]
    fn vog_update_and_photo_server_tags() {
        assert_eq!(ServerTag::parse("VU"), Some(ServerTag::Vu));
        assert_eq!(ServerTag::parse("PH"), Some(ServerTag::Ph));
        assert_eq!(ServerTag::Vu.as_str(), "VU");
        assert_eq!(ServerTag::Ph.as_str(), "PH");
        assert_eq!(format_vog_update(301, 14), "VU\n301 14\n#");
        assert_eq!(
            format_photo_signature(10, 20, PHOTO_DENIED_SIGNATURE),
            "PH\n10 20 DENIED\n#"
        );
    }

    #[test]
    fn pong_server_tag_and_format() {
        assert_eq!(ServerTag::parse("PONG"), Some(ServerTag::Pong));
        assert_eq!(ServerTag::Pong.as_str(), "PONG");
        assert_eq!(format_pong("abc123"), "PONG\nabc123\n#");
    }

    #[test]
    fn baby_wiggle_and_dying_server_tags() {
        assert_eq!(ServerTag::parse("BW"), Some(ServerTag::Bw));
        assert_eq!(ServerTag::parse("DY"), Some(ServerTag::Dy));
        assert_eq!(ServerTag::Bw.as_str(), "BW");
        assert_eq!(ServerTag::Dy.as_str(), "DY");
        assert_eq!(ServerTag::Pe.as_str(), "PE");
    }

    #[test]
    fn learned_tool_and_tool_slots_tags() {
        // Haxe ClientTag: LEARNED_TOOL_REPORT = "LR", TOOL_SLOTS = "TS", LINEAGE = "LN"
        assert_eq!(ServerTag::parse("LR"), Some(ServerTag::Lr));
        assert_eq!(ServerTag::parse("TS"), Some(ServerTag::Ts));
        assert_eq!(ServerTag::parse("TE"), Some(ServerTag::Te));
        assert_eq!(ServerTag::parse("LN"), Some(ServerTag::Ln));
        assert_eq!(ServerTag::Lr.as_str(), "LR");
        assert_eq!(ServerTag::Ts.as_str(), "TS");
        assert_eq!(ServerTag::Ln.as_str(), "LN");
        assert_eq!(format_learned_tool_report(&[12, 334]), "LR\n12 334\n#");
        assert_eq!(format_tool_slots(2, 1000), "TS\n2 1000\n#");
    }

    #[test]
    fn craving_server_tag() {
        assert_eq!(ServerTag::parse("CR"), Some(ServerTag::Cr));
        assert_eq!(ServerTag::Cr.as_str(), "CR");
    }
}
