//! P3#18 integration: speech → getEmotionIndex → outbound EMOT.
//!
//! Unit logic lives in `emotion::classify_speech_outbound`; this file covers
//! the pure table + wire encode paths without a live server.

use ohol_headless::{
    classify_speech_outbound, encode_emot, encode_say, EmotionBank, SpeechOutbound,
};

fn sample_bank() -> EmotionBank {
    EmotionBank::from_ini_strings(
        "/happy\n/mad\n/wave\n",
        "0 1843 0 0 0 0\n1839 1842 0 0 0 0\n0 0 0 0 0 0 2\n",
    )
}

#[test]
fn get_emotion_index_exact_only() {
    let bank = sample_bank();
    assert_eq!(bank.get_emotion_index("/happy"), Some(0));
    assert_eq!(bank.get_emotion_index("/HAPPY"), Some(0));
    assert_eq!(bank.get_emotion_index("/mad"), Some(1));
    assert_eq!(bank.get_emotion_index("/happy now"), None);
    assert_eq!(bank.get_emotion_index("happy"), None);
}

#[test]
fn classify_routes_emot_say_local() {
    let bank = sample_bank();
    match classify_speech_outbound("/wave", &bank) {
        SpeechOutbound::Emot { index, line } => {
            assert_eq!(index, 2);
            assert_eq!(line, "EMOT 0 0 2#");
            assert_eq!(line, encode_emot(0, 0, 2));
        }
        other => panic!("expected Emot, got {other:?}"),
    }
    match classify_speech_outbound("hi there", &bank) {
        SpeechOutbound::Say(line) => {
            assert_eq!(line, encode_say(0, 0, "hi there"));
        }
        other => panic!("expected Say, got {other:?}"),
    }
    assert_eq!(
        classify_speech_outbound("/fps", &bank),
        SpeechOutbound::LocalOnly
    );
}

#[test]
fn real_content_happy_if_present() {
    let root = std::path::Path::new(r"C:\OhOl\OpenLife\OneLifeData7");
    if !root
        .join("contentSettings")
        .join("emotionWords.ini")
        .is_file()
    {
        return;
    }
    let bank = EmotionBank::load_from_content_root(root);
    assert_eq!(bank.get_emotion_index("/happy"), Some(0));
    match classify_speech_outbound("/happy", &bank) {
        SpeechOutbound::Emot { index, line } => {
            assert_eq!(index, 0);
            assert_eq!(line, "EMOT 0 0 0#");
        }
        other => panic!("expected Emot for /happy, got {other:?}"),
    }
}
