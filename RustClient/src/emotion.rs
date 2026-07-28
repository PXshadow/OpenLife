//! Emotion table from `contentSettings/emotionWords.ini` + `emotionObjects.ini`.
//!
//! Chunk **L-EMOT**: PE → `Emotion` object slots (eye/mouth/other/face/body/head)
//! + optional `extraAnimIndex` for `ANIM_EXTRA` gesture packs.
//!
//! C++: `emotion.cpp` / `emotion.h` (`initEmotion`, `getEmotion`,
//! `getEmotionIndex`, `getEmotionObjectByIndex`). Haxe: `Emote.hx` / `EmoteData.hx`.
//!
//! Draw interleave lives in `animationBank.cpp` `drawObjectAnim` via
//! `setAnimationEmotion` / `addExtraAnimationEmotions`.
//!
//! **P3#18** — typed `/happy` etc. → `getEmotionIndex` → outbound `EMOT 0 0 N#`
//! (C++ LivingLifePage say-field `/` command path).
//! **P3#19** — `eyesIndex` / `mainEyesOffset` eyeEmot placement
//! (`content::setup_eyes_and_mouth` + `render` Face phase); PE `extra`↔`extraB`
//! gesture toggle; mouth-sprite skip when `mouthEmot`; creation/decay sounds
//! on PE apply/clear.

use std::fs;
use std::path::{Path, PathBuf};

/// One emotion row (C++ `Emotion` / Haxe `EmoteData`).
///
/// Any object-slot id may be `0` (nothing). `extra_anim_index` is `-1` when
/// the 7th field is omitted (most facial emotes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emotion {
    pub trigger_word: String,
    /// Drawn on top of eyes, offset from head via `mainEyesOffset` (C++).
    pub eye_emot: i32,
    /// Drawn relative to head; replaces mouth.
    pub mouth_emot: i32,
    /// Drawn on top of head sprite.
    pub other_emot: i32,
    /// Drawn on top of eyes (or head if no eyes).
    pub face_emot: i32,
    /// Drawn on body, under top arm / clothing.
    pub body_emot: i32,
    /// Drawn on top of everything (hat-like).
    pub head_emot: i32,
    /// `-1` = no extra animation; else `7xN` extra slot for person.
    pub extra_anim_index: i32,
}

impl Emotion {
    /// C++ `getEmotionNumObjectSlots` → 6.
    pub const NUM_OBJECT_SLOTS: usize = 6;

    /// C++ `getEmotionObjectByIndex`.
    pub fn object_by_index(&self, index: usize) -> i32 {
        match index {
            0 => self.eye_emot,
            1 => self.mouth_emot,
            2 => self.other_emot,
            3 => self.face_emot,
            4 => self.body_emot,
            5 => self.head_emot,
            _ => 0,
        }
    }

    /// All six object-slot ids in C++ order.
    pub fn object_slots(&self) -> [i32; 6] {
        [
            self.eye_emot,
            self.mouth_emot,
            self.other_emot,
            self.face_emot,
            self.body_emot,
            self.head_emot,
        ]
    }

    /// True if any layer object id is set.
    pub fn has_any_object(&self) -> bool {
        self.object_slots().iter().any(|&id| id > 0)
    }

    /// True when PE should switch the person pack to `ANIM_EXTRA`.
    pub fn has_extra_anim(&self) -> bool {
        self.extra_anim_index >= 0
    }

    /// Parse one `emotionObjects.ini` line: up to 7 ints.
    pub fn parse_objects_line(trigger: &str, line: &str) -> Self {
        let mut nums = [0i32; 7];
        nums[6] = -1; // default extraAnimIndex
        for (i, tok) in line.split_whitespace().take(7).enumerate() {
            if let Ok(n) = tok.parse::<i32>() {
                nums[i] = n;
            }
        }
        Self {
            trigger_word: trigger.trim().to_ascii_uppercase(),
            eye_emot: nums[0],
            mouth_emot: nums[1],
            other_emot: nums[2],
            face_emot: nums[3],
            body_emot: nums[4],
            head_emot: nums[5],
            extra_anim_index: nums[6],
        }
    }
}

/// Default temporary emote display time (C++ `emotDuration` setting, default 10).
pub const DEFAULT_EMOT_DURATION_SEC: f32 = 10.0;

/// How typed speech maps to a client→server line (C++ say-field submit).
///
/// // C++: LivingLifePage ~27071–27090 — `/` commands never go as SAY;
/// // exact emotion trigger → `EMOT 0 0 N#`; other `/` stay local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechOutbound {
    /// Normal chat → `SAY 0 0 text#`.
    Say(String),
    /// Exact emotion trigger → `EMOT 0 0 index#`.
    Emot { index: i32, line: String },
    /// Slash command that is not an emote (fps/die/… residual) — no wire.
    LocalOnly,
}

/// Pure classify: speech text + bank → outbound kind (no I/O).
///
/// // C++: if text starts with `/` then getEmotionIndex else SAY path
pub fn classify_speech_outbound(text: &str, bank: &EmotionBank) -> SpeechOutbound {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return SpeechOutbound::LocalOnly;
    }
    if let Some(idx) = bank.get_emotion_index(trimmed) {
        return SpeechOutbound::Emot {
            index: idx,
            line: crate::actions::encode_emot(0, 0, idx),
        };
    }
    if trimmed.starts_with('/') {
        // C++: slash commands are not sent as SAY (local fps/net/die/…).
        return SpeechOutbound::LocalOnly;
    }
    SpeechOutbound::Say(crate::actions::encode_say(0, 0, trimmed))
}

/// Loaded emotion table for PE resolution + draw.
#[derive(Debug, Clone, Default)]
pub struct EmotionBank {
    pub emotions: Vec<Emotion>,
    /// Client-side default TTL when PE omits `ttl_sec` (seconds).
    pub default_duration_sec: f32,
}

impl EmotionBank {
    pub fn new() -> Self {
        Self {
            emotions: Vec::new(),
            default_duration_sec: DEFAULT_EMOT_DURATION_SEC,
        }
    }

    pub fn len(&self) -> usize {
        self.emotions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.emotions.is_empty()
    }

    /// C++ `getEmotion` — `None` if index out of range.
    pub fn get(&self, index: i32) -> Option<&Emotion> {
        if index < 0 {
            return None;
        }
        self.emotions.get(index as usize)
    }

    /// C++ `getEmotionIndex` — first trigger that **exactly** equals uppercased speech.
    ///
    /// // C++ emotion.cpp: upperSpeech starts with triggerWord AND
    /// // `upperSpeech[triggerLen] == '\0'` (nothing after the trigger).
    /// Returns `None` for no match (C++ returns -1).
    pub fn get_emotion_index(&self, speech: &str) -> Option<i32> {
        let upper = speech.trim().to_ascii_uppercase();
        if upper.is_empty() {
            return None;
        }
        for (i, e) in self.emotions.iter().enumerate() {
            let tw = e.trigger_word.as_str();
            if tw.is_empty() {
                continue;
            }
            // Exact match only (C++ starts-with + end-of-string after trigger).
            if upper == tw {
                return Some(i as i32);
            }
        }
        None
    }

    /// Resolve `extraAnimIndex` for a PE index (`None` if missing or `-1`).
    pub fn extra_anim_for(&self, emot_index: i32) -> Option<i32> {
        self.get(emot_index).and_then(|e| {
            if e.extra_anim_index >= 0 {
                Some(e.extra_anim_index)
            } else {
                None
            }
        })
    }

    /// Settings dirs under a OneLifeData7-style content root.
    pub fn settings_dirs_for_root(root: &Path) -> Vec<PathBuf> {
        vec![
            root.join("contentSettings"),
            root.join("settings"),
            // Some trees nest game settings next to objects/
            root.join("gameSource").join("settings"),
        ]
    }

    /// Load words + objects from the first settings dir that has either file.
    pub fn load_from_content_root(root: &Path) -> Self {
        for dir in Self::settings_dirs_for_root(root) {
            if dir.join("emotionObjects.ini").is_file() || dir.join("emotionWords.ini").is_file()
            {
                return Self::load_from_settings_dir(&dir);
            }
        }
        Self::new()
    }

    /// Load from an explicit settings directory.
    pub fn load_from_settings_dir(dir: &Path) -> Self {
        let words_path = dir.join("emotionWords.ini");
        let objs_path = dir.join("emotionObjects.ini");
        let words_raw = fs::read_to_string(&words_path).unwrap_or_default();
        let objs_raw = fs::read_to_string(&objs_path).unwrap_or_default();
        Self::from_ini_strings(&words_raw, &objs_raw)
    }

    /// Parse ini text (unit tests / in-memory).
    pub fn from_ini_strings(words: &str, objects: &str) -> Self {
        let word_lines: Vec<String> = words
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();
        let obj_lines: Vec<&str> = objects
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        let n = obj_lines.len().max(word_lines.len());
        let mut emotions = Vec::with_capacity(n);
        for i in 0..n {
            let trigger = word_lines
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("DUMMY*TRIGGER");
            let obj_line = obj_lines.get(i).copied().unwrap_or("0 0 0 0 0 0");
            emotions.push(Emotion::parse_objects_line(trigger, obj_line));
        }
        // C++: when objects list is longer than words, dummy triggers are already
        // inserted above via DUMMY*TRIGGER.
        Self {
            emotions,
            default_duration_sec: DEFAULT_EMOT_DURATION_SEC,
        }
    }

    /// Prefer-cache style: load from root if present; never bakes (ini only).
    pub fn load_prefer_cache(root: &Path) -> Self {
        Self::load_from_content_root(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bank() -> EmotionBank {
        EmotionBank::from_ini_strings(
            "/happy\n/mad\n/wave\n",
            "0 1843 0 0 0 0\n1839 1842 0 0 0 0\n0 0 0 0 0 0 2\n",
        )
    }

    #[test]
    fn parse_objects_line_six_and_seven_fields() {
        let e = Emotion::parse_objects_line("/happy", "0 1843 0 0 0 0");
        assert_eq!(e.trigger_word, "/HAPPY");
        assert_eq!(e.mouth_emot, 1843);
        assert_eq!(e.extra_anim_index, -1);
        assert!(!e.has_extra_anim());

        let g = Emotion::parse_objects_line("/wave", "0 0 0 0 0 0 2");
        assert_eq!(g.extra_anim_index, 2);
        assert!(g.has_extra_anim());
        assert_eq!(g.object_by_index(1), 0);
    }

    #[test]
    fn bank_from_ini_aligns_words_and_objects() {
        let words = "/happy\n/mad\n/point\n";
        let objs = "0 1843 0 0 0 0\n1839 1842 0 0 0 0\n0 0 0 0 0 0 0\n";
        let bank = EmotionBank::from_ini_strings(words, objs);
        assert_eq!(bank.len(), 3);
        assert_eq!(bank.get(0).unwrap().mouth_emot, 1843);
        assert_eq!(bank.get(1).unwrap().eye_emot, 1839);
        assert_eq!(bank.get(2).unwrap().extra_anim_index, 0);
        assert_eq!(bank.extra_anim_for(0), None);
        assert_eq!(bank.extra_anim_for(2), Some(0));
        assert!(bank.get(99).is_none());
    }

    #[test]
    fn get_emotion_index_exact_trigger_only() {
        let bank = sample_bank();
        // Case-insensitive exact match
        assert_eq!(bank.get_emotion_index("/happy"), Some(0));
        assert_eq!(bank.get_emotion_index("/HAPPY"), Some(0));
        assert_eq!(bank.get_emotion_index("  /Mad  "), Some(1));
        assert_eq!(bank.get_emotion_index("/wave"), Some(2));
        // Prefix / extra text must NOT match (C++ requires end after trigger)
        assert_eq!(bank.get_emotion_index("/happy now"), None);
        assert_eq!(bank.get_emotion_index("happy"), None);
        assert_eq!(bank.get_emotion_index("/happ"), None);
        assert_eq!(bank.get_emotion_index(""), None);
        assert_eq!(bank.get_emotion_index("/unknown"), None);
    }

    #[test]
    fn classify_speech_outbound_emot_say_local() {
        let bank = sample_bank();
        match classify_speech_outbound("/happy", &bank) {
            SpeechOutbound::Emot { index, line } => {
                assert_eq!(index, 0);
                assert_eq!(line, "EMOT 0 0 0#");
            }
            other => panic!("expected Emot, got {other:?}"),
        }
        match classify_speech_outbound("HELLO", &bank) {
            SpeechOutbound::Say(line) => assert_eq!(line, "SAY 0 0 HELLO#"),
            other => panic!("expected Say, got {other:?}"),
        }
        assert_eq!(
            classify_speech_outbound("/fps", &bank),
            SpeechOutbound::LocalOnly
        );
        assert_eq!(
            classify_speech_outbound("", &bank),
            SpeechOutbound::LocalOnly
        );
    }

    #[test]
    fn load_real_content_settings_if_present() {
        let root = Path::new(r"C:\OhOl\OpenLife\OneLifeData7");
        if !root.join("contentSettings").join("emotionObjects.ini").is_file() {
            return;
        }
        let bank = EmotionBank::load_from_content_root(root);
        assert!(bank.len() >= 30, "expected full emotion table, got {}", bank.len());
        // Index 0 = /happy → mouth 1843
        let happy = bank.get(0).expect("happy");
        assert!(happy.mouth_emot > 0 || happy.eye_emot > 0);
        assert_eq!(bank.get_emotion_index("/happy"), Some(0));
        assert_eq!(bank.get_emotion_index("/HAPPY"), Some(0));
        // Gesture rows near end have extraAnimIndex
        let with_extra = bank.emotions.iter().filter(|e| e.has_extra_anim()).count();
        assert!(with_extra >= 1, "expected at least one gesture extra");
    }
}
