// ============================================================
// #255 Structured argument tuple ingestor
//
// Argumentation2.0 / IBM-Debater / Araucaria export arguments as
// JSON with (claim, premises, relation) structures. This parser
// extracts (claim, SupportedBy|AttackedBy, premise) triples — one
// row per (claim × premise) pair.
//
// Canonical predicate names:
//   "support" / "pro"              → SupportedBy
//   "attack" / "con" / "rebut"     → AttackedBy
//   "undermine"                    → Undermines (specific: attack on a premise)
//   "undercut"                     → Undercuts   (specific: attack on the inference)
// ============================================================

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ArgumentRecord {
    pub claim: String,
    #[serde(default)]
    pub premises: Vec<Premise>,
    /// Optional relation at the top level (some corpora attach it here).
    #[serde(default)]
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Premise {
    pub text: String,
    /// Per-premise relation; overrides the top-level one if present.
    #[serde(default)]
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedArgumentTuple {
    pub claim: String,
    pub predicate: &'static str,
    pub premise: String,
    pub raw_relation: String,
}

fn predicate_name(rel: &str) -> Option<&'static str> {
    match rel.trim().to_ascii_lowercase().as_str() {
        "support" | "pro" | "agree" | "in-favor" => Some("SupportedBy"),
        "attack" | "con" | "rebut" | "against" => Some("AttackedBy"),
        "undermine" => Some("Undermines"),
        "undercut" => Some("Undercuts"),
        _ => None,
    }
}

/// Parse one JSON argument record into zero or more (claim, rel, premise)
/// triples — one per non-skipped premise.
pub fn parse_record(json: &str) -> Result<Vec<ParsedArgumentTuple>, serde_json::Error> {
    let record: ArgumentRecord = serde_json::from_str(json.trim())?;

    let claim = record.claim.trim();
    if claim.is_empty() || claim.len() > 512 {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let default_rel = record.relation.as_deref();
    for p in record.premises.iter() {
        let premise_text = p.text.trim();
        if premise_text.is_empty() || premise_text.len() > 512 {
            continue;
        }
        // Per-premise relation wins over the top-level default.
        let raw_rel = p.relation.as_deref().or(default_rel).unwrap_or("support");
        let Some(pred) = predicate_name(raw_rel) else { continue };
        out.push(ParsedArgumentTuple {
            claim: claim.to_string(),
            predicate: pred,
            premise: premise_text.to_string(),
            raw_relation: raw_rel.to_string(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claim_with_multiple_premises() {
        let json = r#"{
            "claim": "Rain should be celebrated.",
            "premises": [
                {"text": "Plants need water.", "relation": "support"},
                {"text": "It interrupts outdoor plans.", "relation": "attack"}
            ]
        }"#;
        let out = parse_record(json).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].predicate, "SupportedBy");
        assert_eq!(out[1].predicate, "AttackedBy");
    }

    #[test]
    fn top_level_relation_is_default() {
        let json = r#"{
            "claim": "X is true.",
            "premises": [{"text": "Y implies X."}, {"text": "Z corroborates X."}],
            "relation": "support"
        }"#;
        let out = parse_record(json).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|t| t.predicate == "SupportedBy"));
    }

    #[test]
    fn per_premise_relation_overrides_default() {
        let json = r#"{
            "claim": "X is true.",
            "premises": [
                {"text": "A", "relation": "attack"},
                {"text": "B"}
            ],
            "relation": "support"
        }"#;
        let out = parse_record(json).unwrap();
        assert_eq!(out[0].predicate, "AttackedBy");
        assert_eq!(out[1].predicate, "SupportedBy");
    }

    #[test]
    fn empty_claim_yields_no_tuples() {
        let json = r#"{"claim":"","premises":[{"text":"A"}]}"#;
        let out = parse_record(json).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn skips_unknown_relation() {
        let json = r#"{
            "claim": "X",
            "premises": [{"text": "A", "relation": "muse_upon"}]
        }"#;
        let out = parse_record(json).unwrap();
        assert!(out.is_empty());
    }
}
