// ============================================================
// LFI Web Search Engine — Real-Time Knowledge Acquisition
//
// Performs actual HTTP requests to search engines and knowledge
// sources, returning structured results with skepticism scoring.
//
// Multi-source cross-referencing: results from multiple sources
// increase trust score. Single-source claims stay low-trust.
//
// All results pass through PSL skepticism gates before ingestion.
// ============================================================

use crate::hdc::error::HdcError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A single search result from a web source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The title of the result.
    pub title: String,
    /// A snippet or summary of the content.
    pub snippet: String,
    /// The source URL.
    pub source_url: String,
    /// Which search backend provided this.
    pub backend: SearchBackend,
    /// Raw trust score for this individual source (0.0 to 1.0).
    pub source_trust: f64,
}

/// Which backend produced the result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchBackend {
    /// DuckDuckGo Instant Answer API.
    DuckDuckGo,
    /// Wikipedia API.
    Wikipedia,
    /// Wiktionary (dictionary definitions).
    Wiktionary,
}

/// Aggregated search response with cross-reference scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// The original query.
    pub query: String,
    /// All results from all backends.
    pub results: Vec<SearchResult>,
    /// Number of distinct sources that returned relevant data.
    pub source_count: usize,
    /// Cross-reference trust score: higher when multiple sources agree.
    /// Formula: T = 1.0 - (1.0 / (1.0 + source_count))
    pub cross_reference_trust: f64,
    /// Best summary extracted across all results.
    pub best_summary: String,
}

/// The web search engine. Queries multiple backends and cross-references.
pub struct WebSearchEngine {
    /// HTTP request timeout.
    timeout: Duration,
    /// Maximum results per backend.
    max_results_per_backend: usize,
}

impl WebSearchEngine {
    /// Create a new web search engine.
    pub fn new() -> Self {
        debuglog!("WebSearchEngine::new: Initializing multi-backend search engine");
        Self {
            timeout: Duration::from_secs(10),
            max_results_per_backend: 5,
        }
    }

    /// Extract the core topic from a natural language question.
    /// "who was the first person on the moon?" → "first person on the moon"
    /// "what is the capital of France?" → "capital of France"
    /// "how does photosynthesis work?" → "photosynthesis"
    fn extract_topic(query: &str) -> String {
        debuglog!("WebSearchEngine::extract_topic: '{}'", query);
        let q = query.to_lowercase();
        let q = q.trim_end_matches('?').trim_end_matches('.').trim();

        // Strip leading question words and articles
        let prefixes = [
            "who was the ", "who is the ", "who are the ",
            "who was ", "who is ", "who are ",
            "what is the ", "what are the ", "what was the ",
            "what is a ", "what is an ", "what is ",
            "what are ", "what was ",
            "how does ", "how do ", "how did ", "how is ", "how are ",
            "how many ", "how much ",
            "where is the ", "where is ", "where are ",
            "when was the ", "when was ", "when did ",
            "why is the ", "why is ", "why are ", "why do ", "why does ",
            "can you tell me about ", "tell me about ",
            "explain ", "define ", "describe ",
        ];

        let mut topic = q.to_string();
        for prefix in &prefixes {
            if topic.starts_with(prefix) {
                topic = topic[prefix.len()..].to_string();
                break;
            }
        }

        // Strip trailing filler
        let suffixes = [" work", " works", " mean", " means",
                        " like", " called", " used for"];
        for suffix in &suffixes {
            if topic.ends_with(suffix) {
                topic = topic[..topic.len() - suffix.len()].to_string();
            }
        }

        let topic = topic.trim().to_string();
        debuglog!("WebSearchEngine::extract_topic: '{}' → '{}'", query, topic);
        topic
    }

    /// Generate multiple search queries from a natural language question.
    /// Returns (topic, [queries]) where topic is the extracted subject
    /// and queries are variations for different backends.
    fn generate_queries(query: &str) -> (String, Vec<String>) {
        let topic = Self::extract_topic(query);

        let mut queries = Vec::new();
        queries.push(topic.clone());

        // For Wikipedia, convert to title case for page lookup
        let wiki_title: String = topic.split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("_");
        if wiki_title != topic {
            queries.push(wiki_title);
        }

        // Also try the full original query for DuckDuckGo
        if query.to_lowercase().trim() != topic {
            queries.push(query.to_string());
        }

        (topic, queries)
    }

    /// Search across all backends and cross-reference results.
    pub fn search(&self, query: &str) -> Result<SearchResponse, HdcError> {
        debuglog!("WebSearchEngine::search: query='{}'", &query[..query.len().min(80)]);

        let (topic, queries) = Self::generate_queries(query);
        debuglog!("WebSearchEngine::search: topic='{}', queries={:?}", topic, queries);
        let mut all_results = Vec::new();

        // Query DuckDuckGo with the extracted topic (works better than full questions)
        for q in &queries {
            match self.search_duckduckgo(q) {
                Ok(results) => {
                    debuglog!("WebSearchEngine::search: DDG('{}')={} results", q, results.len());
                    all_results.extend(results);
                    if !all_results.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    debuglog!("WebSearchEngine::search: DDG('{}') ERROR: {:?}", q, e);
                }
            }
        }

        // Query Wikipedia with the title-cased topic
        for q in &queries {
            match self.search_wikipedia(q) {
                Ok(results) => {
                    debuglog!("WebSearchEngine::search: WIKI('{}')={} results", q, results.len());
                    all_results.extend(results);
                    if !all_results.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    debuglog!("WebSearchEngine::search: WIKI('{}') ERROR: {:?}", q, e);
                }
            }
        }

        // Query Wiktionary for word definitions (only for short topics)
        let topic_words: Vec<&str> = topic.split_whitespace().collect();
        if topic_words.len() <= 3 {
            match self.search_wiktionary(&topic) {
                Ok(results) => {
                    debuglog!("WebSearchEngine::search: Wiktionary returned {} results", results.len());
                    all_results.extend(results);
                }
                Err(e) => {
                    debuglog!("WebSearchEngine::search: Wiktionary FAILED: {:?}", e);
                }
            }
        }

        // Count distinct backends that returned results
        let mut backends_seen = Vec::new();
        for r in &all_results {
            if !backends_seen.contains(&r.backend) {
                backends_seen.push(r.backend.clone());
            }
        }
        let source_count = backends_seen.len();

        // Cross-reference trust: more sources = higher trust.
        // Consensus logic: if snippets from different backends share key technical terms, trust increases.
        let cross_reference_trust = if source_count == 0 {
            0.0
        } else {
            let mut base_trust = 1.0 - (1.0 / (1.0 + source_count as f64));
            
            // Refine trust based on overlapping content
            if source_count > 1 {
                let mut agreement_bonus = 0.0;
                for i in 0..all_results.len() {
                    for j in i+1..all_results.len() {
                        if all_results[i].backend != all_results[j].backend {
                            let overlap = Self::calculate_snippet_overlap(&all_results[i].snippet, &all_results[j].snippet);
                            agreement_bonus += overlap * 0.1;
                        }
                    }
                }
                base_trust = (base_trust + agreement_bonus).min(0.95); // Never 100% trust
            }
            base_trust
        };

        // Extract the best summary (prefer Wikipedia extracts, then DuckDuckGo, then first snippet)
        let best_summary = self.extract_best_summary(&all_results);

        debuglog!(
            "WebSearchEngine::search: {} total results, {} sources, trust={:.2}",
            all_results.len(), source_count, cross_reference_trust
        );

        Ok(SearchResponse {
            query: query.to_string(),
            results: all_results,
            source_count,
            cross_reference_trust,
            best_summary,
        })
    }

    /// Calculate the overlap between two snippets (naive keyword intersection).
    fn calculate_snippet_overlap(s1: &str, s2: &str) -> f64 {
        let s1_low = s1.to_lowercase();
        let s2_low = s2.to_lowercase();
        let w1: std::collections::HashSet<_> = s1_low
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 4)
            .collect();
        let w2: std::collections::HashSet<_> = s2_low
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 4)
            .collect();
        
        if w1.is_empty() || w2.is_empty() { return 0.0; }
        
        let intersection = w1.intersection(&w2).count();
        let union = w1.union(&w2).count();
        
        intersection as f64 / union as f64
    }

    /// Query DuckDuckGo Instant Answer API (no API key required).
    fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>, HdcError> {
        debuglog!("WebSearchEngine::search_duckduckgo: '{}'", query);

        let encoded_query = Self::url_encode(query);
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            encoded_query
        );

        let body = self.http_get(&url)?;

        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            HdcError::InitializationFailed {
                reason: format!("DuckDuckGo JSON parse error: {}", e),
            }
        })?;

        let mut results = Vec::new();

        // Extract Abstract (main answer)
        if let Some(abstract_text) = parsed.get("AbstractText").and_then(|v| v.as_str()) {
            if !abstract_text.is_empty() {
                let source = parsed.get("AbstractSource")
                    .and_then(|v| v.as_str())
                    .unwrap_or("DuckDuckGo");
                let url = parsed.get("AbstractURL")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                results.push(SearchResult {
                    title: query.to_string(),
                    snippet: abstract_text.to_string(),
                    source_url: url.to_string(),
                    backend: SearchBackend::DuckDuckGo,
                    source_trust: if source.contains("Wikipedia") { 0.7 } else { 0.5 },
                });
            }
        }

        // Extract Answer (direct answer for factual queries)
        if let Some(answer) = parsed.get("Answer").and_then(|v| v.as_str()) {
            if !answer.is_empty() {
                results.push(SearchResult {
                    title: format!("Answer: {}", query),
                    snippet: answer.to_string(),
                    source_url: String::new(),
                    backend: SearchBackend::DuckDuckGo,
                    source_trust: 0.6,
                });
            }
        }

        // Extract Definition
        if let Some(definition) = parsed.get("Definition").and_then(|v| v.as_str()) {
            if !definition.is_empty() {
                results.push(SearchResult {
                    title: format!("Definition: {}", query),
                    snippet: definition.to_string(),
                    source_url: parsed.get("DefinitionURL")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    backend: SearchBackend::DuckDuckGo,
                    source_trust: 0.6,
                });
            }
        }

        // Extract Related Topics
        if let Some(related) = parsed.get("RelatedTopics").and_then(|v| v.as_array()) {
            for topic in related.iter().take(self.max_results_per_backend) {
                if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        let first_url = topic.get("FirstURL")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        results.push(SearchResult {
                            title: crate::truncate_str(&text, 80).to_string(),
                            snippet: text.to_string(),
                            source_url: first_url,
                            backend: SearchBackend::DuckDuckGo,
                            source_trust: 0.4,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Query Wikipedia API for article summaries.
    /// First searches for matching pages, then fetches the best match's summary.
    fn search_wikipedia(&self, query: &str) -> Result<Vec<SearchResult>, HdcError> {
        debuglog!("WebSearchEngine::search_wikipedia: '{}'", query);

        // Step 1: Search Wikipedia for matching pages
        let encoded_query = Self::url_encode(query);
        let search_url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&format=json&srlimit=3&utf8=",
            encoded_query
        );

        let mut results = Vec::new();

        if let Ok(search_body) = self.http_get(&search_url) {
            if let Ok(search_parsed) = serde_json::from_str::<serde_json::Value>(&search_body) {
                // Extract page titles from search results
                if let Some(search_results) = search_parsed.get("query")
                    .and_then(|q| q.get("search"))
                    .and_then(|s| s.as_array()) {

                    for sr in search_results.iter().take(2) {
                        if let Some(title) = sr.get("title").and_then(|t| t.as_str()) {
                            // Step 2: Fetch the summary for this specific page
                            // Wikipedia REST API uses underscores for spaces, not +
                            let title_wiki = title.replace(' ', "_");
                            let title_encoded = Self::url_encode(&title_wiki);
                            let summary_url = format!(
                                "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
                                title_encoded
                            );

                            if let Ok(summary_body) = self.http_get(&summary_url) {
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&summary_body) {
                                    if let Some(extract) = parsed.get("extract").and_then(|v| v.as_str()) {
                                        if !extract.is_empty() && extract.len() > 20 {
                                            let page_url = parsed.get("content_urls")
                                                .and_then(|v| v.get("desktop"))
                                                .and_then(|v| v.get("page"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");

                                            results.push(SearchResult {
                                                title: title.to_string(),
                                                snippet: extract.to_string(),
                                                source_url: page_url.to_string(),
                                                backend: SearchBackend::Wikipedia,
                                                source_trust: 0.65,
                                            });
                                            // Got a good result, don't need more
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: try direct page lookup if search didn't work
        if results.is_empty() {
            let wiki_query = query.replace(' ', "_");
            let direct_url = format!(
                "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
                Self::url_encode(&wiki_query)
            );

            if let Ok(body) = self.http_get(&direct_url) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(extract) = parsed.get("extract").and_then(|v| v.as_str()) {
                        if !extract.is_empty() && extract.len() > 20 {
                            let title = parsed.get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(query);
                            let page_url = parsed.get("content_urls")
                                .and_then(|v| v.get("desktop"))
                                .and_then(|v| v.get("page"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            results.push(SearchResult {
                                title: title.to_string(),
                                snippet: extract.to_string(),
                                source_url: page_url.to_string(),
                                backend: SearchBackend::Wikipedia,
                                source_trust: 0.65,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Query Wiktionary for word definitions.
    fn search_wiktionary(&self, query: &str) -> Result<Vec<SearchResult>, HdcError> {
        debuglog!("WebSearchEngine::search_wiktionary: '{}'", query);

        let word = query.split_whitespace()
            .last()
            .unwrap_or(query)
            .to_lowercase();

        let url = format!(
            "https://en.wiktionary.org/api/rest_v1/page/definition/{}",
            Self::url_encode(&word)
        );

        let body = self.http_get(&url)?;

        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            HdcError::InitializationFailed {
                reason: format!("Wiktionary JSON parse error: {}", e),
            }
        })?;

        let mut results = Vec::new();

        // Parse definitions from Wiktionary response
        if let Some(en) = parsed.get("en").and_then(|v| v.as_array()) {
            for entry in en.iter().take(3) {
                if let Some(definitions) = entry.get("definitions").and_then(|v| v.as_array()) {
                    for def in definitions.iter().take(2) {
                        if let Some(definition) = def.get("definition").and_then(|v| v.as_str()) {
                            // Strip HTML tags
                            let clean = Self::strip_html(definition);
                            if !clean.is_empty() {
                                let part_of_speech = entry.get("partOfSpeech")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                results.push(SearchResult {
                                    title: format!("{} ({})", word, part_of_speech),
                                    snippet: clean,
                                    source_url: format!("https://en.wiktionary.org/wiki/{}", word),
                                    backend: SearchBackend::Wiktionary,
                                    source_trust: 0.6,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Perform an HTTP GET request.
    fn http_get(&self, url: &str) -> Result<String, HdcError> {
        debuglog!("WebSearchEngine::http_get: {}", &url[..url.len().min(120)]);

        let response = ureq::get(url)
            .timeout(self.timeout)
            .set("User-Agent", "LFI-Sovereign-Intelligence/1.0 (Research)")
            .set("Accept", "application/json")
            .call()
            .map_err(|e| {
                debuglog!("WebSearchEngine::http_get: FAILED: {}", e);
                HdcError::InitializationFailed {
                    reason: format!("HTTP request failed: {}", e),
                }
            })?;

        let status = response.status();
        let body = response.into_string().map_err(|e| HdcError::InitializationFailed {
            reason: format!("HTTP response read failed: {}", e),
        })?;

        debuglog!("WebSearchEngine::http_get: status={}, body_len={}", status, body.len());
        Ok(body)
    }

    /// URL-encode a query string.
    fn url_encode(input: &str) -> String {
        let mut encoded = String::new();
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => encoded.push('+'),
                _ => {
                    encoded.push('%');
                    encoded.push_str(&format!("{:02X}", byte));
                }
            }
        }
        encoded
    }

    /// Strip HTML tags from a string.
    fn strip_html(input: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        for ch in input.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }
        result.trim().to_string()
    }

    /// Extract the best summary from search results.
    fn extract_best_summary(&self, results: &[SearchResult]) -> String {
        debuglog!("WebSearchEngine::extract_best_summary: from {} results", results.len());

        if results.is_empty() {
            return String::new();
        }

        // Prefer Wikipedia extracts (longer, more authoritative)
        for r in results {
            if r.backend == SearchBackend::Wikipedia && r.snippet.len() > 50 {
                return r.snippet.clone();
            }
        }

        // Then DuckDuckGo abstracts
        for r in results {
            if r.backend == SearchBackend::DuckDuckGo && r.snippet.len() > 30 {
                return r.snippet.clone();
            }
        }

        // Then Wiktionary definitions
        for r in results {
            if r.backend == SearchBackend::Wiktionary {
                return r.snippet.clone();
            }
        }

        // Fallback: first result
        results[0].snippet.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(WebSearchEngine::url_encode("hello world"), "hello+world");
        assert_eq!(WebSearchEngine::url_encode("rust programming"), "rust+programming");
        assert_eq!(WebSearchEngine::url_encode("what is 2+2"), "what+is+2%2B2");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(
            WebSearchEngine::strip_html("<b>bold</b> and <i>italic</i>"),
            "bold and italic"
        );
        assert_eq!(
            WebSearchEngine::strip_html("no tags here"),
            "no tags here"
        );
    }

    #[test]
    fn test_search_engine_creation() {
        let engine = WebSearchEngine::new();
        assert_eq!(engine.max_results_per_backend, 5);
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(WebSearchEngine::url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(WebSearchEngine::url_encode("test?query"), "test%3Fquery");
    }

    #[test]
    fn test_strip_html_nested() {
        let result = WebSearchEngine::strip_html("<div><p>Hello <b>World</b></p></div>");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(WebSearchEngine::strip_html(""), "");
        assert_eq!(WebSearchEngine::strip_html("<br/>"), "");
    }

    #[test]
    fn test_search_result_structure() {
        let result = SearchResult {
            title: "Test Result".into(),
            source_url: "https://example.com".into(),
            snippet: "A test snippet".into(),
            backend: SearchBackend::DuckDuckGo,
            source_trust: 0.8,
        };
        assert_eq!(result.title, "Test Result");
        assert_eq!(result.backend, SearchBackend::DuckDuckGo);
    }

    #[test]
    fn test_search_response_empty() {
        let response = SearchResponse {
            query: "nonexistent query".into(),
            results: vec![],
            source_count: 0,
            best_summary: String::new(),
            cross_reference_trust: 0.0,
        };
        assert!(response.results.is_empty());
        assert_eq!(response.cross_reference_trust, 0.0);
    }
}
