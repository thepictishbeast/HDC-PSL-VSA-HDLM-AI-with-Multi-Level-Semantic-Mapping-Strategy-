// ============================================================
// #343 HDLM Tier-1 forensic parser for English
//
// Full dependency + constituency parsing requires a trained parser
// model (spaCy / Stanza / benepar), which is out of scope for an
// in-tree scaffold. This module ships the BASELINE — tokenisation,
// heuristic POS tags, and a shallow clause structure — wired into
// the existing hdlm::Ast arena so downstream HDC encoding has
// something to consume today. The trained-parser swap-in is a
// drop-in replacement for `pos_tag` + `build_tree`.
// ============================================================

use crate::hdlm::ast::{Ast, NodeId, NodeKind};
use crate::hdlm::error::HdlmError;

/// Coarse POS tag. Kept intentionally small — enough to distinguish
/// the roles a downstream encoder cares about. A fuller tagset
/// (Penn / Universal Dependencies) would plug in where these are
/// assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pos {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Determiner,
    Preposition,
    Conjunction,
    Punctuation,
    Other,
}

impl Pos {
    pub fn as_str(&self) -> &'static str {
        match self {
            Pos::Noun => "NOUN",
            Pos::Verb => "VERB",
            Pos::Adjective => "ADJ",
            Pos::Adverb => "ADV",
            Pos::Determiner => "DET",
            Pos::Preposition => "PREP",
            Pos::Conjunction => "CONJ",
            Pos::Punctuation => "PUNCT",
            Pos::Other => "X",
        }
    }
}

/// One tokenised piece of input.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub pos: Pos,
}

/// Whitespace + punctuation-aware tokenisation. Splits on whitespace,
/// breaks out trailing punctuation as separate tokens.
pub fn tokenise(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for raw in text.split_whitespace() {
        // Peel trailing . , ; : ! ?
        let mut word = raw;
        let trailing: Vec<char> = word.chars().rev()
            .take_while(|c| matches!(*c, '.'|','|';'|':'|'!'|'?'))
            .collect();
        if !trailing.is_empty() {
            let cut = word.len() - trailing.len();
            word = &word[..cut];
        }
        if !word.is_empty() {
            out.push(Token {
                text: word.to_lowercase(),
                pos: pos_tag(word),
            });
        }
        // Emit punctuation tokens in original order.
        for punct in trailing.iter().rev() {
            out.push(Token {
                text: punct.to_string(),
                pos: Pos::Punctuation,
            });
        }
    }
    out
}

/// Heuristic POS tagger. Closed-class lookup first, then suffix rules.
/// Trained-parser replacement would replace this single function.
pub fn pos_tag(word: &str) -> Pos {
    let w = word.to_lowercase();
    // Closed classes — finite small sets.
    const DETERMINERS: &[&str] = &[
        "a", "an", "the", "this", "that", "these", "those", "my",
        "your", "his", "her", "its", "our", "their", "some", "any",
    ];
    const PREPOSITIONS: &[&str] = &[
        "in", "on", "at", "by", "for", "with", "about", "against",
        "between", "into", "through", "during", "before", "after",
        "above", "below", "to", "from", "of", "off", "over", "under",
    ];
    const CONJUNCTIONS: &[&str] = &[
        "and", "or", "but", "so", "because", "although", "while",
        "if", "unless", "since", "though", "as", "than",
    ];
    const COMMON_VERBS: &[&str] = &[
        "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did",
        "can", "could", "will", "would", "shall", "should", "may",
        "might", "must", "ought",
    ];
    if DETERMINERS.contains(&w.as_str()) { return Pos::Determiner; }
    if PREPOSITIONS.contains(&w.as_str()) { return Pos::Preposition; }
    if CONJUNCTIONS.contains(&w.as_str()) { return Pos::Conjunction; }
    if COMMON_VERBS.contains(&w.as_str()) { return Pos::Verb; }

    // Suffix heuristics. Ordering matters: check longer / more
    // specific suffixes before shorter ones.
    if w.ends_with("ly") { return Pos::Adverb; }
    if w.ends_with("ing") || w.ends_with("ed") { return Pos::Verb; }
    if w.ends_with("ous") || w.ends_with("ful") || w.ends_with("ish")
        || w.ends_with("able") || w.ends_with("ible")
        || w.ends_with("al") || w.ends_with("ive") {
        return Pos::Adjective;
    }
    if w.ends_with("tion") || w.ends_with("ment") || w.ends_with("ness")
        || w.ends_with("ity") || w.ends_with("er")
        || w.ends_with("ism") {
        return Pos::Noun;
    }
    if w.ends_with('s') && w.len() > 3 {
        // Weak heuristic: plural nouns / third-person-singular verbs.
        // Default to Noun; fuller parser would use context.
        return Pos::Noun;
    }

    // Default: treat unknown words as nouns (English majority class).
    Pos::Noun
}

/// Build a shallow AST from tokens: Root → Sentence → Phrase for
/// each whitespace-delimited word. Punctuation collapses to sentence
/// boundaries. Downstream encoders can re-read the Phrase texts and
/// per-phrase POS tags via the token stream.
pub fn build_tree(ast: &mut Ast, tokens: &[Token]) -> Result<NodeId, HdlmError> {
    let root = ast.add_node(NodeKind::Root);
    let mut current_sentence: Option<NodeId> = None;

    for tok in tokens {
        // Sentence-final punctuation closes the current sentence.
        if tok.pos == Pos::Punctuation
            && matches!(tok.text.as_str(), "." | "!" | "?")
        {
            current_sentence = None;
            continue;
        }
        let sent = match current_sentence {
            Some(s) => s,
            None => {
                let s = ast.add_node(NodeKind::Sentence);
                ast.add_child(root, s).map_err(|_| HdlmError::MalformedAst {
                    reason: "add_child sentence failed".into(),
                })?;
                current_sentence = Some(s);
                s
            }
        };
        let phrase = ast.add_node(NodeKind::Phrase {
            text: tok.text.clone(),
        });
        ast.add_child(sent, phrase).map_err(|_| HdlmError::MalformedAst {
            reason: "add_child phrase failed".into(),
        })?;
    }
    Ok(root)
}

/// One-shot: text → AST.
pub fn parse_english(text: &str) -> Result<(Ast, NodeId), HdlmError> {
    let tokens = tokenise(text);
    let mut ast = Ast::new();
    let root = build_tree(&mut ast, &tokens)?;
    Ok((ast, root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_splits_whitespace_and_punctuation() {
        let toks = tokenise("The quick fox jumps over the lazy dog.");
        assert_eq!(toks.len(), 9); // 8 words + final period
        assert_eq!(toks[0].text, "the");
        assert_eq!(toks[0].pos, Pos::Determiner);
        assert_eq!(toks[8].text, ".");
        assert_eq!(toks[8].pos, Pos::Punctuation);
    }

    #[test]
    fn pos_tag_closed_classes() {
        assert_eq!(pos_tag("the"), Pos::Determiner);
        assert_eq!(pos_tag("AND"), Pos::Conjunction);
        assert_eq!(pos_tag("in"), Pos::Preposition);
        assert_eq!(pos_tag("is"), Pos::Verb);
    }

    #[test]
    fn pos_tag_suffix_heuristics() {
        assert_eq!(pos_tag("running"), Pos::Verb);
        assert_eq!(pos_tag("quickly"), Pos::Adverb);
        assert_eq!(pos_tag("dangerous"), Pos::Adjective);
        assert_eq!(pos_tag("revolution"), Pos::Noun);
    }

    #[test]
    fn pos_tag_unknown_word_defaults_to_noun() {
        assert_eq!(pos_tag("xyzzy"), Pos::Noun);
    }

    #[test]
    fn build_tree_produces_root_and_sentence() {
        let toks = tokenise("Water is wet.");
        let mut ast = Ast::new();
        let root = build_tree(&mut ast, &toks).unwrap();
        // Root has one sentence child.
        let root_node = ast.get_node(root).unwrap();
        assert_eq!(root_node.children.len(), 1);
        // Sentence has 3 phrases (water, is, wet).
        let sent = ast.get_node(root_node.children[0]).unwrap();
        assert_eq!(sent.children.len(), 3);
    }

    #[test]
    fn build_tree_handles_multiple_sentences() {
        let toks = tokenise("Rain falls. Plants grow.");
        let mut ast = Ast::new();
        let root = build_tree(&mut ast, &toks).unwrap();
        let root_node = ast.get_node(root).unwrap();
        // Two sentences.
        assert_eq!(root_node.children.len(), 2);
    }

    #[test]
    fn parse_english_one_shot() {
        let (ast, root) = parse_english("The cat sits.").unwrap();
        let r = ast.get_node(root).unwrap();
        assert_eq!(r.children.len(), 1);
    }
}
