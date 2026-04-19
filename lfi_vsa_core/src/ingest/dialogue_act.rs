// ============================================================
// #335 Dialogue-act decomposer
//
// OASST / UltraChat / Switchboard-DAMSL tag every turn with a
// dialogue act (greeting, question, statement, agreement, ...).
// This parser consumes records of the shape
//
//   { turn_id, speaker, text, act }
//
// and yields (speaker, act_predicate, text) triples with the act
// mapped to a canonical predicate name used across the rest of the
// pipeline (#345 speech-act classifier consumes the same vocabulary).
// ============================================================

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueTurnRecord {
    #[serde(default)]
    pub turn_id: Option<String>,
    pub speaker: String,
    pub text: String,
    pub act: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedDialogueTuple {
    pub speaker: String,
    pub act: &'static str,
    pub text: String,
    pub turn_id: Option<String>,
    pub conversation_id: Option<String>,
    pub raw_act: String,
}

/// Map a raw act label to the canonical vocabulary shared with the
/// runtime speech-act classifier (cognition/speech_act.rs).
fn canonical_act(label: &str) -> Option<&'static str> {
    // Match case-insensitively + strip Switchboard-DAMSL prefixes
    // like "sd" (statement, declarative).
    let l = label.trim().to_ascii_lowercase();
    Some(match l.as_str() {
        "greet" | "greeting" | "hi" | "hello" => "Greet",
        "farewell" | "bye" | "closing" => "Farewell",
        "statement" | "sd" | "assert" | "inform" => "Statement",
        "question" | "q" | "yq" | "wq" | "ask" => "Question",
        "answer" | "ans" | "ny" | "nn" | "confirm" => "Answer",
        "agree" | "aa" | "acknowledge" | "acc" => "Agree",
        "disagree" | "ar" | "reject" => "Disagree",
        "thank" | "thanks" | "ft" => "Thank",
        "apology" | "fa" | "sorry" => "Apologize",
        "request" | "req" | "command" | "cmd" => "Request",
        "suggest" | "propose" => "Suggest",
        "clarify" | "clarification" => "Clarify",
        "opinion" | "sv" => "Opinion",
        _ => return None,
    })
}

/// Parse one JSON record into a single dialogue tuple.
pub fn parse_record(json: &str) -> Result<Option<ParsedDialogueTuple>, serde_json::Error> {
    let record: DialogueTurnRecord = serde_json::from_str(json.trim())?;
    let speaker = record.speaker.trim();
    let text = record.text.trim();
    if speaker.is_empty() || text.is_empty()
        || speaker.len() > 64 || text.len() > 2048 {
        return Ok(None);
    }
    let Some(canonical) = canonical_act(&record.act) else {
        return Ok(None);
    };
    Ok(Some(ParsedDialogueTuple {
        speaker: speaker.to_string(),
        act: canonical,
        text: text.to_string(),
        turn_id: record.turn_id,
        conversation_id: record.conversation_id,
        raw_act: record.act,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_question_turn() {
        let json = r#"{"speaker":"user","text":"What is water?","act":"question"}"#;
        let p = parse_record(json).unwrap().unwrap();
        assert_eq!(p.speaker, "user");
        assert_eq!(p.act, "Question");
        assert_eq!(p.text, "What is water?");
    }

    #[test]
    fn maps_switchboard_damsl_codes() {
        let json = r#"{"speaker":"A","text":"Okay.","act":"aa"}"#;
        let p = parse_record(json).unwrap().unwrap();
        assert_eq!(p.act, "Agree");
    }

    #[test]
    fn preserves_turn_and_conversation_id() {
        let json = r#"{
            "turn_id":"t42","conversation_id":"c7",
            "speaker":"U","text":"hi","act":"greet"
        }"#;
        let p = parse_record(json).unwrap().unwrap();
        assert_eq!(p.turn_id.as_deref(), Some("t42"));
        assert_eq!(p.conversation_id.as_deref(), Some("c7"));
    }

    #[test]
    fn case_insensitive_act() {
        let json = r#"{"speaker":"X","text":"Yes.","act":"AGREE"}"#;
        let p = parse_record(json).unwrap().unwrap();
        assert_eq!(p.act, "Agree");
    }

    #[test]
    fn empty_text_returns_none() {
        let json = r#"{"speaker":"X","text":"","act":"statement"}"#;
        assert!(parse_record(json).unwrap().is_none());
    }

    #[test]
    fn unknown_act_returns_none() {
        let json = r#"{"speaker":"X","text":"Y","act":"soliloquy"}"#;
        assert!(parse_record(json).unwrap().is_none());
    }

    #[test]
    fn raw_act_is_preserved() {
        let json = r#"{"speaker":"X","text":"hi","act":"greet"}"#;
        let p = parse_record(json).unwrap().unwrap();
        assert_eq!(p.raw_act, "greet");
    }
}
