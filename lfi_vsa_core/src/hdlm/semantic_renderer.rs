// ============================================================
// #344 Semantic surface renderer
//
// Given a composite BipolarVector + a vocabulary (SymbolicCodebook
// from #342), return text describing its top-K semantic components.
// This is the output side of the HDLM pipeline — the step that
// converts a 10000-dim hypervector back into language.
//
// It isn't a full natural-language generator yet (that's the deeper
// #344 work — composing rendered concepts into a grammatical
// sentence). This piece is the decoder: confidence-ranked concept
// matches. Downstream callers (ForensicGenerator, DecorativeExpander)
// can compose them into prose using their existing AST → text path.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdlm::SymbolicCodebook;

/// One ranked match. `symbol` is the codebook key; `score` is the
/// cosine similarity in [-1, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticMatch {
    pub symbol: String,
    pub score: f64,
}

/// Reverse-lookup the top-K symbols most similar to `composite`.
///
/// BUG ASSUMPTION: `candidates` is the full vocabulary to scan. For a
/// 50k-synset codebook, this is O(|candidates|) vector comparisons —
/// acceptable at build-time, expensive in a tight loop. Callers with
/// high throughput should pre-filter candidates (e.g. by domain / POS).
pub fn nearest_symbols(
    composite: &BipolarVector,
    codebook: &SymbolicCodebook,
    candidates: &[&str],
    k: usize,
) -> Vec<SemanticMatch> {
    if candidates.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut scored: Vec<SemanticMatch> = candidates.iter().map(|s| {
        let v = codebook.encode(s);
        let score = composite.similarity(&v).unwrap_or(0.0);
        SemanticMatch { symbol: s.to_string(), score }
    }).collect();

    // Partial sort: descend by score, keep top K. For huge candidate
    // sets a heap would be better; at vocab sizes we actually see,
    // a full sort is fine.
    scored.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    scored
}

/// Render a composite hypervector as a text sketch: top-K matches
/// joined with a delimiter and annotated with their scores.
/// "water(0.98) + liquid(0.81) + drink(0.73)"
pub fn render_sketch(
    composite: &BipolarVector,
    codebook: &SymbolicCodebook,
    candidates: &[&str],
    k: usize,
    min_score: f64,
) -> String {
    let matches = nearest_symbols(composite, codebook, candidates, k);
    let filtered: Vec<String> = matches.into_iter()
        .filter(|m| m.score >= min_score)
        .map(|m| format!("{}({:.2})", m.symbol, m.score))
        .collect();
    if filtered.is_empty() {
        "<no confident match>".to_string()
    } else {
        filtered.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_recovers_the_stored_symbol() {
        // Encode "water", then ask for the nearest — should be self.
        let cb = SymbolicCodebook::new("test");
        let water_vec = cb.encode("water");
        let matches = nearest_symbols(
            &water_vec, &cb,
            &["water", "fire", "rock", "tree"],
            3,
        );
        assert_eq!(matches[0].symbol, "water");
        assert!(matches[0].score > 0.99);
    }

    #[test]
    fn nearest_handles_empty_candidates() {
        let cb = SymbolicCodebook::new("test");
        let v = cb.encode("anything");
        assert!(nearest_symbols(&v, &cb, &[], 5).is_empty());
    }

    #[test]
    fn sketch_reports_confident_matches() {
        let cb = SymbolicCodebook::new("test");
        let water = cb.encode("water");
        let sketch = render_sketch(
            &water, &cb, &["water", "fire", "rock"], 3, 0.5,
        );
        assert!(sketch.contains("water"));
    }

    #[test]
    fn sketch_low_confidence_returns_placeholder() {
        let cb = SymbolicCodebook::new("test");
        let v = BipolarVector::from_seed(999_999);
        let sketch = render_sketch(
            &v, &cb, &["unrelated_a", "unrelated_b"], 3, 0.5,
        );
        // Orthogonal vectors scored ≪ 0.5 → placeholder.
        assert_eq!(sketch, "<no confident match>");
    }

    #[test]
    fn top_k_is_respected() {
        let cb = SymbolicCodebook::new("test");
        let v = cb.encode("x");
        let matches = nearest_symbols(
            &v, &cb, &["a", "b", "c", "d", "e"], 2,
        );
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn scores_are_sorted_descending() {
        let cb = SymbolicCodebook::new("test");
        let v = cb.encode("x");
        let matches = nearest_symbols(
            &v, &cb, &["x", "a", "b", "c"], 4,
        );
        for pair in matches.windows(2) {
            assert!(pair[0].score >= pair[1].score,
                    "scores out of order: {:?}", matches);
        }
    }
}
