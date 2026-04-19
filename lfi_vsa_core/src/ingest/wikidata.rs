// ============================================================
// #200 Wikidata entity parser
//
// The full Wikidata dump (latest-all.json.bz2, ~100 GB) is a line-
// delimited JSON stream of entities. This module is the parser half:
// given one entity blob, emit the structured data we care about for
// brain.db:
//
//   - labels: [(concept_id, language, text)] for concept_translations
//   - instance_of / subclass_of / part_of: (subj, pred, obj) triples
//     for facts_tuples
//
// The STREAMING half (decompress, iterate, batch-insert) lives in a
// separate binary (tools/lfi-wikidata-ingest) because it's a long-
// running job that needs its own process boundary. This module is
// the deterministic per-entity transformation, easy to unit-test.
//
// Wikidata property IDs we care about:
//   P31  instance of
//   P279 subclass of
//   P361 part of
//   P527 has part
//   P1889 different from
// ============================================================

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WikidataEntity {
    pub id: String,
    #[serde(default)]
    pub labels: std::collections::HashMap<String, WikidataLabel>,
    #[serde(default)]
    pub claims: std::collections::HashMap<String, Vec<WikidataClaim>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WikidataLabel {
    pub language: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WikidataClaim {
    #[serde(default)]
    pub mainsnak: Option<WikidataSnak>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WikidataSnak {
    #[serde(default)]
    pub datavalue: Option<WikidataDataValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WikidataDataValue {
    #[serde(default, rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub value: serde_json::Value,
}

/// Result of parsing one entity.
#[derive(Debug, Clone, Default)]
pub struct ParsedEntity {
    pub entity_id: String,
    /// (language, text) pairs to link to entity_id in concept_translations.
    pub labels: Vec<(String, String)>,
    /// Structured relations (subject=entity_id, predicate, object).
    pub relations: Vec<(String, String)>, // (predicate_name, target_entity_id)
}

/// Map a Wikidata property id to a canonical predicate name. Returns
/// None for properties we don't capture (skipping is cheaper than
/// carrying raw Pxxx through the pipeline).
fn predicate_name(pid: &str) -> Option<&'static str> {
    match pid {
        "P31"   => Some("IsA"),           // instance of
        "P279"  => Some("SubclassOf"),    // subclass of
        "P361"  => Some("PartOf"),        // part of
        "P527"  => Some("HasPart"),       // has part
        "P1889" => Some("DifferentFrom"), // different from
        _ => None,
    }
}

/// Parse one JSON line (the newline-delimited Wikidata dump format).
/// Returns None when the line is a list-wrapper element like `[` or
/// `]` or ends with `,` — those are legal in the concatenated array
/// form of the dump and aren't errors.
pub fn parse_line(line: &str) -> Result<Option<ParsedEntity>, serde_json::Error> {
    let trimmed = line.trim().trim_end_matches(',');
    if trimmed.is_empty() || trimmed == "[" || trimmed == "]" {
        return Ok(None);
    }
    let entity: WikidataEntity = serde_json::from_str(trimmed)?;

    let mut out = ParsedEntity {
        entity_id: entity.id.clone(),
        ..Default::default()
    };

    // Labels → concept_translations rows. Cap at 24 languages per
    // entity to keep batches bounded (there are ~300 total; the head
    // dozen covers 95% of downstream use).
    for (lang, label) in entity.labels.iter().take(24) {
        if lang.is_empty() || label.value.is_empty() { continue; }
        // Defensive length cap — pathological entities exist.
        if label.value.len() > 256 { continue; }
        out.labels.push((lang.clone(), label.value.clone()));
    }

    // Claims → (predicate, target) pairs. We only look at the subset
    // mapped by predicate_name.
    for (pid, claims) in entity.claims.iter() {
        let Some(pname) = predicate_name(pid) else { continue };
        for claim in claims.iter().take(32) {
            let Some(snak) = claim.mainsnak.as_ref() else { continue };
            let Some(dv) = snak.datavalue.as_ref() else { continue };
            if dv.type_ != "wikibase-entityid" { continue; }
            // value is {entity-type: "item", id: "Q..."}
            if let Some(target_id) = dv.value.get("id").and_then(|v| v.as_str()) {
                out.relations.push((pname.to_string(), target_id.to_string()));
            }
        }
    }

    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entity_with_labels_and_claims() {
        let json = r#"{
            "id": "Q42",
            "labels": {
                "en": {"language": "en", "value": "Douglas Adams"},
                "es": {"language": "es", "value": "Douglas Adams"}
            },
            "claims": {
                "P31": [{
                    "mainsnak": {
                        "datavalue": {
                            "type": "wikibase-entityid",
                            "value": {"entity-type": "item", "id": "Q5"}
                        }
                    }
                }]
            }
        }"#;
        let parsed = parse_line(json).unwrap().unwrap();
        assert_eq!(parsed.entity_id, "Q42");
        assert_eq!(parsed.labels.len(), 2);
        assert_eq!(parsed.relations.len(), 1);
        assert_eq!(parsed.relations[0].0, "IsA");
        assert_eq!(parsed.relations[0].1, "Q5");
    }

    #[test]
    fn parse_skips_bracket_lines() {
        assert!(parse_line("[").unwrap().is_none());
        assert!(parse_line("]").unwrap().is_none());
        assert!(parse_line("").unwrap().is_none());
    }

    #[test]
    fn parse_trims_trailing_comma() {
        let line = r#"{"id":"Q1","labels":{},"claims":{}},"#;
        let parsed = parse_line(line).unwrap().unwrap();
        assert_eq!(parsed.entity_id, "Q1");
    }

    #[test]
    fn parse_ignores_unknown_properties() {
        let json = r#"{
            "id": "Q1",
            "labels": {},
            "claims": {
                "P9999": [{
                    "mainsnak": {
                        "datavalue": {
                            "type": "wikibase-entityid",
                            "value": {"entity-type": "item", "id": "Q2"}
                        }
                    }
                }]
            }
        }"#;
        let parsed = parse_line(json).unwrap().unwrap();
        assert!(parsed.relations.is_empty(),
                "unknown property should be skipped, got {:?}", parsed.relations);
    }

    #[test]
    fn parse_caps_labels_per_entity() {
        // 30 langs in — expect 24 out (hardcoded cap).
        let mut langs = String::new();
        for i in 0..30 {
            if i > 0 { langs.push(','); }
            langs.push_str(&format!(
                r#""lang{}":{{"language":"lang{}","value":"v{}"}}"#, i, i, i
            ));
        }
        let json = format!(r#"{{"id":"Q1","labels":{{{}}},"claims":{{}}}}"#, langs);
        let parsed = parse_line(&json).unwrap().unwrap();
        assert_eq!(parsed.labels.len(), 24);
    }
}
