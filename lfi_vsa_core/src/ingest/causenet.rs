// ============================================================
// #331 CauseNet entity parser
//
// CauseNet ships as JSON-lines with the shape:
//   {"causal_relation": {"cause": {"concept": "X"},
//                        "effect": {"concept": "Y"}},
//    "sources": [{"type": "wikipedia_infobox", ...}, ...],
//    "sentence": "X causes Y."}
//
// This parser extracts the (cause, Causes, effect) triple and
// source count per entry so the streaming binary can land them in
// facts_tuples (#329) + fact_edges (#336) without further
// transformation.
// ============================================================

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CauseNetRecord {
    pub causal_relation: CausalRelation,
    #[serde(default)]
    pub sources: Vec<CauseNetSource>,
    #[serde(default)]
    pub sentence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CausalRelation {
    pub cause: Concept,
    pub effect: Concept,
}

#[derive(Debug, Deserialize)]
pub struct Concept {
    pub concept: String,
}

#[derive(Debug, Deserialize)]
pub struct CauseNetSource {
    #[serde(default, rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedCausalEdge {
    pub cause: String,
    pub effect: String,
    /// "Causes" canonical predicate — matches the enum in
    /// cognition/knowledge_graph.rs + facts_tuples values.
    pub predicate: &'static str,
    /// Source count; a rough confidence proxy — more sources means
    /// the relation is corroborated.
    pub source_count: usize,
}

/// Parse one JSON-lines record. Empty / malformed lines return
/// Ok(None) so the streaming loop can log-and-skip.
pub fn parse_line(line: &str) -> Result<Option<ParsedCausalEdge>, serde_json::Error> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let rec: CauseNetRecord = serde_json::from_str(trimmed)?;

    let cause = rec.causal_relation.cause.concept.trim().to_lowercase();
    let effect = rec.causal_relation.effect.concept.trim().to_lowercase();
    if cause.is_empty() || effect.is_empty() { return Ok(None); }
    if cause.len() > 128 || effect.len() > 128 { return Ok(None); }

    Ok(Some(ParsedCausalEdge {
        cause,
        effect,
        predicate: "Causes",
        source_count: rec.sources.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimum_valid_record() {
        let json = r#"{
            "causal_relation": {
                "cause": {"concept": "smoking"},
                "effect": {"concept": "lung cancer"}
            },
            "sources": [{"type": "wikipedia"}],
            "sentence": "Smoking causes lung cancer."
        }"#;
        let parsed = parse_line(json).unwrap().unwrap();
        assert_eq!(parsed.cause, "smoking");
        assert_eq!(parsed.effect, "lung cancer");
        assert_eq!(parsed.predicate, "Causes");
        assert_eq!(parsed.source_count, 1);
    }

    #[test]
    fn normalises_case_and_trim() {
        let json = r#"{
            "causal_relation": {
                "cause": {"concept": "  Rain  "},
                "effect": {"concept": "Flooding"}
            },
            "sources": []
        }"#;
        let parsed = parse_line(json).unwrap().unwrap();
        assert_eq!(parsed.cause, "rain");
        assert_eq!(parsed.effect, "flooding");
    }

    #[test]
    fn skips_empty_concepts() {
        let json = r#"{
            "causal_relation": {
                "cause": {"concept": ""},
                "effect": {"concept": "something"}
            },
            "sources": []
        }"#;
        assert!(parse_line(json).unwrap().is_none());
    }

    #[test]
    fn skips_blank_lines() {
        assert!(parse_line("").unwrap().is_none());
        assert!(parse_line("  \n").unwrap().is_none());
    }

    #[test]
    fn source_count_aggregates() {
        let json = r#"{
            "causal_relation": {
                "cause": {"concept": "heat"},
                "effect": {"concept": "expansion"}
            },
            "sources": [
                {"type": "wikipedia_infobox"},
                {"type": "clueweb"},
                {"type": "reuters"}
            ]
        }"#;
        let parsed = parse_line(json).unwrap().unwrap();
        assert_eq!(parsed.source_count, 3);
    }
}
