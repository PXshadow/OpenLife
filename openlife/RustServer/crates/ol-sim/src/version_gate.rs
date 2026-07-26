//! Client / data version gate (**stub** — pure decision only).
//!
//! OHOL clients learn the required data version from the SN greeting
//! (`required_version`, typically from `dataVersionNumber.txt`). This module
//! classifies whether a connecting client is acceptable before LOGIN bootstrap.
//!
//! Not yet wired into `ol-net` accept / LOGIN paths — callers may use
//! [`check_client_version`] to reject mismatches.

/// Default required data version when content has not been loaded (matches
/// `server.toml` / config default 437).
pub const DEFAULT_REQUIRED_VERSION: i32 = 437;

/// Outcome of a client version check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionGateResult {
    /// Client may proceed to LOGIN bootstrap.
    Allow,
    /// Client data version does not match the server requirement.
    RejectVersionMismatch {
        client: i32,
        required: i32,
    },
    /// Client version was missing / unparsable and policy requires an exact match.
    RejectMissingVersion,
    /// Optional soft path: client is newer than server (stub — currently Allow
    /// when equality not required; reserved for future policy).
    AllowClientNewer {
        client: i32,
        required: i32,
    },
}

impl VersionGateResult {
    pub fn is_allowed(self) -> bool {
        matches!(
            self,
            Self::Allow | Self::AllowClientNewer { .. }
        )
    }

    /// Short wire / log reason token.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Allow => "ok",
            Self::RejectVersionMismatch { .. } => "version_mismatch",
            Self::RejectMissingVersion => "missing_version",
            Self::AllowClientNewer { .. } => "client_newer",
        }
    }
}

/// Policy knobs for the version gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionGatePolicy {
    /// Server required data version (SN field).
    pub required: i32,
    /// If true, client must report an exact match (strict).
    pub require_exact: bool,
    /// If true and client > required, allow with [`VersionGateResult::AllowClientNewer`].
    pub allow_newer: bool,
    /// If true, missing client version is rejected; if false, missing → Allow.
    pub require_client_version: bool,
}

impl Default for VersionGatePolicy {
    fn default() -> Self {
        Self {
            required: DEFAULT_REQUIRED_VERSION,
            require_exact: true,
            allow_newer: false,
            require_client_version: false,
        }
    }
}

impl VersionGatePolicy {
    pub fn strict(required: i32) -> Self {
        Self {
            required,
            require_exact: true,
            allow_newer: false,
            require_client_version: true,
        }
    }

    pub fn permissive(required: i32) -> Self {
        Self {
            required,
            require_exact: false,
            allow_newer: true,
            require_client_version: false,
        }
    }
}

/// Check a client-reported data version against policy.
///
/// `client_version`: `None` if the client did not send one (LOGIN has no version
/// field today — gate is primarily for future / tooling / SN echo checks).
pub fn check_client_version(
    client_version: Option<i32>,
    policy: &VersionGatePolicy,
) -> VersionGateResult {
    let Some(client) = client_version else {
        return if policy.require_client_version {
            VersionGateResult::RejectMissingVersion
        } else {
            VersionGateResult::Allow
        };
    };

    if client == policy.required {
        return VersionGateResult::Allow;
    }

    if client > policy.required && policy.allow_newer {
        return VersionGateResult::AllowClientNewer {
            client,
            required: policy.required,
        };
    }

    if policy.require_exact || client < policy.required {
        return VersionGateResult::RejectVersionMismatch {
            client,
            required: policy.required,
        };
    }

    // Non-exact, not older, allow_newer false: still reject mismatch.
    VersionGateResult::RejectVersionMismatch {
        client,
        required: policy.required,
    }
}

/// Convenience: exact match against `required` (None client allowed).
pub fn versions_compatible(client: Option<i32>, required: i32) -> bool {
    check_client_version(
        client,
        &VersionGatePolicy {
            required,
            require_exact: true,
            allow_newer: false,
            require_client_version: false,
        },
    )
    .is_allowed()
}

/// Parse a version token from SN-style text or a raw integer string.
pub fn parse_version_token(s: &str) -> Option<i32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.parse().ok()
}

/// `VERSION required=N client=N|? status=ok|version_mismatch|…` body.
pub fn format_version_gate_query(
    policy: &VersionGatePolicy,
    client: Option<i32>,
) -> String {
    let result = check_client_version(client, policy);
    let client_s = client
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".into());
    format!(
        "VERSION required={} client={} status={}",
        policy.required,
        client_s,
        result.reason()
    )
}

/// Stub reject message for REJECTED-style server reply / logs.
pub fn format_version_reject_message(result: VersionGateResult) -> Option<String> {
    match result {
        VersionGateResult::RejectVersionMismatch { client, required } => Some(format!(
            "REJECTED version client={client} required={required}"
        )),
        VersionGateResult::RejectMissingVersion => {
            Some("REJECTED version missing".into())
        }
        _ => None,
    }
}

/// Pre-login PS body line (`p_id` 0): `0 VERSION REJECTED client=N required=N`.
///
/// Used when [`crate::SimState::client_version_strict`] hard-rejects LOGIN.
pub fn format_version_reject_ps(result: VersionGateResult) -> Option<String> {
    match result {
        VersionGateResult::RejectVersionMismatch { client, required } => Some(format!(
            "0 VERSION REJECTED client={client} required={required}"
        )),
        VersionGateResult::RejectMissingVersion => {
            Some("0 VERSION REJECTED missing".into())
        }
        _ => None,
    }
}

/// Whether a hard-reject LOGIN should fire under `client_version_strict`.
///
/// Only true when the client reported a version and the gate rejects it (or
/// missing version when policy requires one). Soft path leaves LOGIN allowed.
pub fn should_hard_reject_login(
    client_version: Option<i32>,
    policy: &VersionGatePolicy,
    client_version_strict: bool,
) -> Option<VersionGateResult> {
    if !client_version_strict {
        return None;
    }
    let result = check_client_version(client_version, policy);
    if result.is_allowed() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_allows() {
        let p = VersionGatePolicy::strict(437);
        assert_eq!(
            check_client_version(Some(437), &p),
            VersionGateResult::Allow
        );
        assert!(versions_compatible(Some(437), 437));
    }

    #[test]
    fn mismatch_rejects_strict() {
        let p = VersionGatePolicy::strict(437);
        assert_eq!(
            check_client_version(Some(400), &p),
            VersionGateResult::RejectVersionMismatch {
                client: 400,
                required: 437
            }
        );
        assert!(!check_client_version(Some(400), &p).is_allowed());
    }

    #[test]
    fn missing_policy() {
        let strict = VersionGatePolicy::strict(437);
        assert_eq!(
            check_client_version(None, &strict),
            VersionGateResult::RejectMissingVersion
        );
        let soft = VersionGatePolicy::default();
        assert_eq!(
            check_client_version(None, &soft),
            VersionGateResult::Allow
        );
        assert!(versions_compatible(None, 437));
    }

    #[test]
    fn allow_newer_path() {
        let p = VersionGatePolicy {
            required: 437,
            require_exact: false,
            allow_newer: true,
            require_client_version: false,
        };
        assert_eq!(
            check_client_version(Some(500), &p),
            VersionGateResult::AllowClientNewer {
                client: 500,
                required: 437
            }
        );
        assert!(check_client_version(Some(500), &p).is_allowed());
        // Older still rejected
        assert!(!check_client_version(Some(400), &p).is_allowed());
    }

    #[test]
    fn parse_and_format() {
        assert_eq!(parse_version_token("437"), Some(437));
        assert_eq!(parse_version_token("  87 "), Some(87));
        assert_eq!(parse_version_token(""), None);
        assert_eq!(parse_version_token("abc"), None);

        let p = VersionGatePolicy::default();
        let q = format_version_gate_query(&p, Some(437));
        assert!(q.contains("required=437"), "{q}");
        assert!(q.contains("status=ok"), "{q}");

        let bad = check_client_version(Some(1), &VersionGatePolicy::strict(437));
        let msg = format_version_reject_message(bad).unwrap();
        assert!(msg.contains("REJECTED"), "{msg}");
        assert!(format_version_reject_message(VersionGateResult::Allow).is_none());
        let ps = format_version_reject_ps(bad).unwrap();
        assert!(ps.starts_with("0 VERSION REJECTED"), "{ps}");
        assert!(ps.contains("client=1"), "{ps}");
        assert!(ps.contains("required=437"), "{ps}");
    }

    #[test]
    fn default_required_constant() {
        assert_eq!(DEFAULT_REQUIRED_VERSION, 437);
        assert_eq!(VersionGatePolicy::default().required, 437);
    }

    #[test]
    fn hard_reject_only_when_strict() {
        let p = VersionGatePolicy::default();
        assert!(should_hard_reject_login(Some(400), &p, false).is_none());
        let r = should_hard_reject_login(Some(400), &p, true).unwrap();
        assert!(!r.is_allowed());
        assert!(should_hard_reject_login(Some(437), &p, true).is_none());
        // Missing version + default policy (require_client_version=false) → allow
        assert!(should_hard_reject_login(None, &p, true).is_none());
        // Missing + strict policy requiring client version → hard reject
        let strict = VersionGatePolicy::strict(437);
        assert!(should_hard_reject_login(None, &strict, true).is_some());
    }
}
