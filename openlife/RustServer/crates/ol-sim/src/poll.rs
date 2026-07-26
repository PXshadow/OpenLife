//! Session-local yes/no poll (pure — no wire/SQL).
//!
//! `SAY POLL <text>` opens (or replaces) a single active poll.
//! `SAY VOTE yes|no` records one vote per player (re-vote overwrites).
//! `SAY ?POLL` returns tallies + question.

use std::collections::HashMap;

/// Yes / no ballot choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteChoice {
    Yes,
    No,
}

impl VoteChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
        }
    }
}

/// Parse `yes` / `no` / `y` / `n` (case-insensitive).
pub fn parse_vote_choice(s: &str) -> Option<VoteChoice> {
    match s.trim().to_ascii_lowercase().as_str() {
        "yes" | "y" => Some(VoteChoice::Yes),
        "no" | "n" => Some(VoteChoice::No),
        _ => None,
    }
}

/// Single active session poll with yes/no tallies.
#[derive(Debug, Clone, Default)]
pub struct PollState {
    /// Poll question text (None = no active poll).
    pub question: Option<String>,
    /// Creator player id when a poll is active.
    pub creator: Option<i32>,
    /// One vote per `p_id` (last vote wins).
    votes: HashMap<i32, VoteChoice>,
}

impl PollState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.question.is_some()
    }

    /// Open a new poll (replaces any previous). Empty / whitespace-only text fails.
    pub fn create(&mut self, creator: i32, question: &str) -> Result<(), &'static str> {
        let q = question.trim();
        if q.is_empty() || creator == 0 {
            return Err("BAD");
        }
        self.question = Some(q.to_string());
        self.creator = Some(creator);
        self.votes.clear();
        Ok(())
    }

    /// Cast or change a vote. Fails when no poll is open.
    pub fn vote(&mut self, p_id: i32, choice: VoteChoice) -> Result<(), &'static str> {
        if !self.is_active() || p_id == 0 {
            return Err("BAD");
        }
        self.votes.insert(p_id, choice);
        Ok(())
    }

    /// `(yes_count, no_count)`.
    pub fn counts(&self) -> (u32, u32) {
        let mut yes = 0u32;
        let mut no = 0u32;
        for c in self.votes.values() {
            match c {
                VoteChoice::Yes => yes += 1,
                VoteChoice::No => no += 1,
            }
        }
        (yes, no)
    }

    pub fn vote_of(&self, p_id: i32) -> Option<VoteChoice> {
        self.votes.get(&p_id).copied()
    }

    pub fn voter_count(&self) -> usize {
        self.votes.len()
    }

    /// Drop the active poll and all votes.
    pub fn clear(&mut self) {
        self.question = None;
        self.creator = None;
        self.votes.clear();
    }

    /// Event-log line for a newly created poll: `POLL {creator} {question}`.
    pub fn format_create_event(creator: i32, question: &str) -> String {
        format!("POLL {creator} {}", question.trim())
    }

    /// Chat body for `SAY ?POLL` without leading p_id.
    ///
    /// - inactive: `POLL none`
    /// - active: `POLL yes=N no=M q={question}`
    pub fn format_query(&self) -> String {
        match &self.question {
            None => "POLL none".into(),
            Some(q) => {
                let (yes, no) = self.counts();
                format!("POLL yes={yes} no={no} q={q}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_vote_counts_and_query() {
        let mut p = PollState::new();
        assert!(!p.is_active());
        assert_eq!(p.format_query(), "POLL none");
        assert!(p.create(1, "  Build wall?  ").is_ok());
        assert!(p.is_active());
        assert_eq!(p.question.as_deref(), Some("Build wall?"));
        assert_eq!(p.creator, Some(1));
        assert!(p.vote(2, VoteChoice::Yes).is_ok());
        assert!(p.vote(3, VoteChoice::No).is_ok());
        assert!(p.vote(4, VoteChoice::Yes).is_ok());
        assert_eq!(p.counts(), (2, 1));
        assert_eq!(p.voter_count(), 3);
        let q = p.format_query();
        assert!(q.contains("yes=2"));
        assert!(q.contains("no=1"));
        assert!(q.contains("q=Build wall?"));
    }

    #[test]
    fn revote_overwrites() {
        let mut p = PollState::new();
        p.create(1, "Go north?").unwrap();
        p.vote(2, VoteChoice::Yes).unwrap();
        assert_eq!(p.counts(), (1, 0));
        p.vote(2, VoteChoice::No).unwrap();
        assert_eq!(p.counts(), (0, 1));
        assert_eq!(p.vote_of(2), Some(VoteChoice::No));
        assert_eq!(p.voter_count(), 1);
    }

    #[test]
    fn create_rejects_empty_and_vote_needs_poll() {
        let mut p = PollState::new();
        assert_eq!(p.create(1, "   "), Err("BAD"));
        assert_eq!(p.create(0, "ok"), Err("BAD"));
        assert_eq!(p.vote(1, VoteChoice::Yes), Err("BAD"));
        p.create(1, "ok").unwrap();
        assert_eq!(p.vote(0, VoteChoice::Yes), Err("BAD"));
    }

    #[test]
    fn create_replaces_previous() {
        let mut p = PollState::new();
        p.create(1, "A").unwrap();
        p.vote(2, VoteChoice::Yes).unwrap();
        p.create(3, "B").unwrap();
        assert_eq!(p.question.as_deref(), Some("B"));
        assert_eq!(p.creator, Some(3));
        assert_eq!(p.counts(), (0, 0));
        assert_eq!(p.voter_count(), 0);
    }

    #[test]
    fn parse_vote_and_event_line() {
        assert_eq!(parse_vote_choice("yes"), Some(VoteChoice::Yes));
        assert_eq!(parse_vote_choice("Y"), Some(VoteChoice::Yes));
        assert_eq!(parse_vote_choice("no"), Some(VoteChoice::No));
        assert_eq!(parse_vote_choice("N"), Some(VoteChoice::No));
        assert_eq!(parse_vote_choice("maybe"), None);
        assert_eq!(
            PollState::format_create_event(7, "  lunch?  "),
            "POLL 7 lunch?"
        );
    }

    #[test]
    fn clear_resets() {
        let mut p = PollState::new();
        p.create(1, "q").unwrap();
        p.vote(2, VoteChoice::Yes).unwrap();
        p.clear();
        assert!(!p.is_active());
        assert_eq!(p.format_query(), "POLL none");
    }
}
