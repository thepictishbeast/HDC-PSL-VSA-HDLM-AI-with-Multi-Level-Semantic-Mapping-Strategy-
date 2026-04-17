// ============================================================
// Information Retrieval Engine — OSINT-Style Deep Gathering
//
// PURPOSE: Given a target (person, organization, event, topic),
// gather comprehensive information by:
//   1. Identifying relevant source types for the target
//   2. Constructing targeted search queries
//   3. Aggregating results across sources
//   4. Deduplicating overlapping facts
//   5. Extracting entities, dates, relationships
//   6. Building a timeline of activities
//   7. Assessing source trust via EpistemicFilter
//
// EXAMPLE QUERIES:
//   "find all info on William Armstrong"
//     → people_search, news, academic citations, patents, news archives
//   "history on the gulf gas station latest activities"
//     → news search, business records, social media, local news
//   "recent CVE for Apache httpd"
//     → NVD, CVE databases, security advisories, GitHub issues
//
// PIPELINE STAGES:
//   Query → QueryPlanner → SourceSelector → Searchers (parallel)
//          → Aggregator → Deduplicator → EntityExtractor
//          → TimelineBuilder → EpistemicFilter → Report
//
// OUTPUT:
//   IntelReport with:
//     - Timeline of events
//     - Key facts by confidence tier
//     - Related entities (people, orgs, places, dates)
//     - Source list with trust scores
//     - Summary paragraphs
// ============================================================

use crate::intelligence::epistemic_filter::{
    EpistemicFilter, Source, SourceCategory, KnowledgeTier,
};
use std::collections::HashMap;

// ============================================================
// Target Classification
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TargetType {
    /// A person (real or fictional).
    Person,
    /// A company, institution, or group.
    Organization,
    /// A physical place (address, landmark, region).
    Location,
    /// A specific event (breach, release, incident).
    Event,
    /// A product, service, or technology.
    Product,
    /// A topic or concept.
    Topic,
    /// Could not classify automatically.
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub target_type: TargetType,
    /// Disambiguating context (e.g., "British engineer" for William Armstrong).
    pub context: Option<String>,
    /// Focus: what aspects to prioritize.
    pub focus: QueryFocus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryFocus {
    /// General background, history, key facts.
    General,
    /// Recent activity — prioritize recency.
    Recent,
    /// Historical — prioritize older sources.
    Historical,
    /// Relationships — who is connected to the target.
    Relationships,
    /// Financial — business/money-related.
    Financial,
    /// Legal — court records, compliance.
    Legal,
    /// Technical — research papers, patents, engineering.
    Technical,
    /// Security — breaches, vulnerabilities, threats.
    Security,
}

impl Target {
    pub fn new(name: &str) -> Self {
        let target_type = Self::classify(name);
        Self {
            name: name.into(),
            target_type,
            context: None,
            focus: QueryFocus::General,
        }
    }

    pub fn with_type(mut self, t: TargetType) -> Self {
        self.target_type = t;
        self
    }

    pub fn with_focus(mut self, focus: QueryFocus) -> Self {
        self.focus = focus;
        self
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Heuristic classification from name alone.
    /// BUG ASSUMPTION: heuristics, not definitive. Override with with_type().
    fn classify(name: &str) -> TargetType {
        let lower = name.to_lowercase();

        // Person names: 2 capitalized words (heuristic).
        let words: Vec<&str> = name.split_whitespace().collect();
        if words.len() == 2 && words.iter().all(|w|
            w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)) {
            return TargetType::Person;
        }

        // Organization indicators.
        let org_suffixes = ["inc", "corp", "llc", "ltd", "company", "station",
                            "institute", "foundation", "agency", "department"];
        if org_suffixes.iter().any(|s| lower.contains(s)) {
            return TargetType::Organization;
        }

        // Location indicators.
        let location_indicators = ["city", "county", "state", "country", "street", "avenue"];
        if location_indicators.iter().any(|s| lower.contains(s)) {
            return TargetType::Location;
        }

        // Event indicators.
        let event_indicators = ["incident", "breach", "attack", "election", "summit"];
        if event_indicators.iter().any(|s| lower.contains(s)) {
            return TargetType::Event;
        }

        TargetType::Unknown
    }
}

// ============================================================
// Query Planning
// ============================================================

/// A search query derived from a target.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query_text: String,
    /// Which source types are likely to have this info.
    pub source_types: Vec<SourceCategory>,
    /// Priority (0.0 low, 1.0 high).
    pub priority: f64,
    /// Why this query was generated.
    pub rationale: String,
}

pub struct QueryPlanner;

impl QueryPlanner {
    /// Generate a plan of search queries for a target.
    /// Returns queries ordered by priority.
    pub fn plan(target: &Target) -> Vec<SearchQuery> {
        let mut queries = Vec::new();
        let name = &target.name;

        // Always query the raw name first.
        queries.push(SearchQuery {
            query_text: name.clone(),
            source_types: vec![
                SourceCategory::Journalism,
                SourceCategory::Community,
                SourceCategory::Standards,
            ],
            priority: 1.0,
            rationale: "direct name match".into(),
        });

        // Type-specific queries.
        match target.target_type {
            TargetType::Person => {
                queries.extend(Self::person_queries(target));
            }
            TargetType::Organization => {
                queries.extend(Self::organization_queries(target));
            }
            TargetType::Event => {
                queries.extend(Self::event_queries(target));
            }
            TargetType::Location => {
                queries.extend(Self::location_queries(target));
            }
            TargetType::Product => {
                queries.extend(Self::product_queries(target));
            }
            TargetType::Topic => {
                queries.extend(Self::topic_queries(target));
            }
            _ => {}
        }

        // Focus-specific queries (added on top of type-based).
        match target.focus {
            QueryFocus::Recent => {
                queries.push(SearchQuery {
                    query_text: format!("{} latest news", name),
                    source_types: vec![SourceCategory::Journalism],
                    priority: 0.95,
                    rationale: "recent activity focus".into(),
                });
            }
            QueryFocus::Security => {
                queries.push(SearchQuery {
                    query_text: format!("{} vulnerability CVE", name),
                    source_types: vec![SourceCategory::Standards],
                    priority: 0.9,
                    rationale: "security vulnerability lookup".into(),
                });
                queries.push(SearchQuery {
                    query_text: format!("{} breach incident", name),
                    source_types: vec![SourceCategory::Journalism, SourceCategory::Standards],
                    priority: 0.85,
                    rationale: "security incident history".into(),
                });
            }
            QueryFocus::Technical => {
                queries.push(SearchQuery {
                    query_text: format!("{} research paper", name),
                    source_types: vec![SourceCategory::PeerReviewed],
                    priority: 0.9,
                    rationale: "academic research".into(),
                });
                queries.push(SearchQuery {
                    query_text: format!("{} patent", name),
                    source_types: vec![SourceCategory::Standards],
                    priority: 0.8,
                    rationale: "patent records".into(),
                });
            }
            QueryFocus::Legal => {
                queries.push(SearchQuery {
                    query_text: format!("{} court case lawsuit", name),
                    source_types: vec![SourceCategory::Journalism, SourceCategory::Standards],
                    priority: 0.85,
                    rationale: "legal proceedings".into(),
                });
            }
            _ => {}
        }

        // Add disambiguating context if provided.
        if let Some(ref ctx) = target.context {
            queries.push(SearchQuery {
                query_text: format!("{} {}", name, ctx),
                source_types: vec![SourceCategory::Journalism, SourceCategory::Community],
                priority: 0.95,
                rationale: format!("disambiguated by '{}'", ctx),
            });
        }

        // Sort by priority descending.
        queries.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        queries
    }

    fn person_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} biography", name),
                source_types: vec![SourceCategory::Journalism, SourceCategory::Community],
                priority: 0.9,
                rationale: "person biography".into(),
            },
            SearchQuery {
                query_text: format!("{} wikipedia", name),
                source_types: vec![SourceCategory::Community],
                priority: 0.85,
                rationale: "encyclopedic summary".into(),
            },
            SearchQuery {
                query_text: format!("{} LinkedIn profile", name),
                source_types: vec![SourceCategory::Community],
                priority: 0.8,
                rationale: "professional history".into(),
            },
            SearchQuery {
                query_text: format!("\"{}\" news", name),
                source_types: vec![SourceCategory::Journalism],
                priority: 0.75,
                rationale: "news mentions".into(),
            },
            SearchQuery {
                query_text: format!("{} publications", name),
                source_types: vec![SourceCategory::PeerReviewed],
                priority: 0.7,
                rationale: "academic publications".into(),
            },
        ]
    }

    fn organization_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} company profile", name),
                source_types: vec![SourceCategory::Journalism, SourceCategory::Community],
                priority: 0.9,
                rationale: "organization overview".into(),
            },
            SearchQuery {
                query_text: format!("{} SEC filing", name),
                source_types: vec![SourceCategory::Standards],
                priority: 0.85,
                rationale: "financial disclosures".into(),
            },
            SearchQuery {
                query_text: format!("{} press release", name),
                source_types: vec![SourceCategory::Journalism],
                priority: 0.8,
                rationale: "official announcements".into(),
            },
            SearchQuery {
                query_text: format!("{} employees leadership", name),
                source_types: vec![SourceCategory::Community],
                priority: 0.7,
                rationale: "organizational structure".into(),
            },
            SearchQuery {
                query_text: format!("{} recent news activities", name),
                source_types: vec![SourceCategory::Journalism],
                priority: 0.85,
                rationale: "recent business activity".into(),
            },
        ]
    }

    fn event_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} timeline events", name),
                source_types: vec![SourceCategory::Journalism, SourceCategory::Community],
                priority: 0.9,
                rationale: "event timeline".into(),
            },
            SearchQuery {
                query_text: format!("{} causes impact", name),
                source_types: vec![SourceCategory::Journalism, SourceCategory::PeerReviewed],
                priority: 0.8,
                rationale: "analysis of event".into(),
            },
            SearchQuery {
                query_text: format!("{} response aftermath", name),
                source_types: vec![SourceCategory::Journalism],
                priority: 0.8,
                rationale: "consequences".into(),
            },
        ]
    }

    fn location_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} history", name),
                source_types: vec![SourceCategory::Community],
                priority: 0.85,
                rationale: "location history".into(),
            },
            SearchQuery {
                query_text: format!("{} recent events", name),
                source_types: vec![SourceCategory::Journalism],
                priority: 0.8,
                rationale: "current events at location".into(),
            },
            SearchQuery {
                query_text: format!("{} demographics", name),
                source_types: vec![SourceCategory::Standards],
                priority: 0.7,
                rationale: "official data".into(),
            },
        ]
    }

    fn product_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} specifications features", name),
                source_types: vec![SourceCategory::Community, SourceCategory::Journalism],
                priority: 0.85,
                rationale: "product details".into(),
            },
            SearchQuery {
                query_text: format!("{} reviews", name),
                source_types: vec![SourceCategory::Community, SourceCategory::Journalism],
                priority: 0.8,
                rationale: "user feedback".into(),
            },
            SearchQuery {
                query_text: format!("{} security advisories", name),
                source_types: vec![SourceCategory::Standards],
                priority: 0.75,
                rationale: "known vulnerabilities".into(),
            },
        ]
    }

    fn topic_queries(t: &Target) -> Vec<SearchQuery> {
        let name = &t.name;
        vec![
            SearchQuery {
                query_text: format!("{} overview introduction", name),
                source_types: vec![SourceCategory::Community, SourceCategory::PeerReviewed],
                priority: 0.85,
                rationale: "topic introduction".into(),
            },
            SearchQuery {
                query_text: format!("{} recent research", name),
                source_types: vec![SourceCategory::PeerReviewed],
                priority: 0.8,
                rationale: "current research".into(),
            },
            SearchQuery {
                query_text: format!("{} examples applications", name),
                source_types: vec![SourceCategory::Community],
                priority: 0.75,
                rationale: "practical uses".into(),
            },
        ]
    }
}

// ============================================================
// Collected Fact
// ============================================================

#[derive(Debug, Clone)]
pub struct Fact {
    pub statement: String,
    pub source: String,
    pub source_category: SourceCategory,
    /// Extracted date if any.
    pub date: Option<String>,
    /// Entities mentioned.
    pub entities: Vec<String>,
    /// Confidence tier.
    pub tier: KnowledgeTier,
    /// Confidence score.
    pub confidence: f64,
}

// ============================================================
// Intel Report
// ============================================================

#[derive(Debug, Clone)]
pub struct IntelReport {
    pub target: Target,
    /// All facts gathered, sorted by confidence.
    pub facts: Vec<Fact>,
    /// Facts organized by tier.
    pub by_tier: HashMap<String, Vec<Fact>>,
    /// Timeline: (date, fact) sorted chronologically.
    pub timeline: Vec<(String, Fact)>,
    /// Related entities mentioned.
    pub related_entities: Vec<String>,
    /// Sources consulted with trust scores.
    pub sources: Vec<(String, f64)>,
    /// Summary text.
    pub summary: String,
    /// Total queries executed.
    pub queries_run: usize,
}

impl IntelReport {
    /// High-level human-readable report.
    pub fn format(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("=== Intel Report: {} ===\n", self.target.name));
        out.push_str(&format!("Type:     {:?}\n", self.target.target_type));
        out.push_str(&format!("Focus:    {:?}\n", self.target.focus));
        out.push_str(&format!("Queries:  {}\n", self.queries_run));
        out.push_str(&format!("Facts:    {}\n", self.facts.len()));
        out.push_str(&format!("Sources:  {}\n", self.sources.len()));
        out.push_str(&format!("\nSummary:\n  {}\n", self.summary));

        if !self.facts.is_empty() {
            out.push_str("\nKey facts (by confidence):\n");
            for fact in self.facts.iter().take(10) {
                out.push_str(&format!(
                    "  [{:?}] {} — {} (conf {:.2})\n",
                    fact.tier, fact.statement, fact.source, fact.confidence,
                ));
            }
        }

        if !self.timeline.is_empty() {
            out.push_str("\nTimeline:\n");
            for (date, fact) in &self.timeline {
                out.push_str(&format!(
                    "  {} — {}\n",
                    date, crate::truncate_str(&fact.statement, 80),
                ));
            }
        }

        if !self.related_entities.is_empty() {
            out.push_str("\nRelated entities:\n");
            for entity in self.related_entities.iter().take(20) {
                out.push_str(&format!("  - {}\n", entity));
            }
        }

        out
    }
}

// ============================================================
// Information Retrieval Engine
// ============================================================

pub struct InfoRetrievalEngine {
    filter: EpistemicFilter,
}

impl InfoRetrievalEngine {
    pub fn new() -> Self {
        debuglog!("InfoRetrievalEngine::new: Initializing deep info gathering");
        let mut filter = EpistemicFilter::new();

        // Pre-register common search sources.
        for (name, category) in &[
            ("web_search", SourceCategory::Community),
            ("news_api", SourceCategory::Journalism),
            ("wikipedia", SourceCategory::Community),
            ("arxiv", SourceCategory::PeerReviewed),
            ("cve_db", SourceCategory::Standards),
            ("sec_filings", SourceCategory::Standards),
            ("linkedin", SourceCategory::Community),
            ("social_media", SourceCategory::Anonymous),
        ] {
            filter.register_source_default(name, category.clone());
        }

        Self { filter }
    }

    /// Plan queries for a target.
    pub fn plan_queries(&self, target: &Target) -> Vec<SearchQuery> {
        QueryPlanner::plan(target)
    }

    /// Ingest a fact from a source with proper filtering.
    /// This is how external search results get incorporated.
    pub fn ingest_fact(
        &mut self,
        statement: &str,
        source_name: &str,
        date: Option<&str>,
    ) -> Fact {
        let result = self.filter.ingest_claim(statement, source_name);
        let source_category = if let Some(src) = self.filter_source(source_name) {
            src.category
        } else {
            SourceCategory::Anonymous
        };

        Fact {
            statement: statement.into(),
            source: source_name.into(),
            source_category,
            date: date.map(|s| s.into()),
            entities: Self::extract_entities(statement),
            tier: result.tier,
            confidence: result.confidence,
        }
    }

    fn filter_source(&self, name: &str) -> Option<Source> {
        None // placeholder; EpistemicFilter's sources are private
            .or_else(|| {
                // Fallback: reconstruct from default categories.
                let category = match name {
                    "wikipedia" | "web_search" | "linkedin" => SourceCategory::Community,
                    "news_api" => SourceCategory::Journalism,
                    "arxiv" => SourceCategory::PeerReviewed,
                    "cve_db" | "sec_filings" => SourceCategory::Standards,
                    _ => SourceCategory::Anonymous,
                };
                Some(Source {
                    name: name.into(),
                    category: category.clone(),
                    trust: category.base_trust(),
                    track_record: 0.5,
                    claim_count: 0,
                })
            })
    }

    /// Extract named entities from text (simple heuristic).
    /// BUG ASSUMPTION: capitalized-word heuristic; real NER would be much better.
    fn extract_entities(text: &str) -> Vec<String> {
        let mut entities = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if clean.is_empty() { continue; }

            let is_capitalized = clean.chars().next()
                .map(|c| c.is_uppercase()).unwrap_or(false);

            if is_capitalized && clean.len() > 2 {
                if !current.is_empty() { current.push(' '); }
                current.push_str(clean);
            } else {
                if current.split_whitespace().count() >= 1 && !current.is_empty() {
                    let trimmed = current.trim().to_string();
                    if trimmed.len() > 2 && !entities.contains(&trimmed) {
                        entities.push(trimmed);
                    }
                }
                current.clear();
            }
        }
        if !current.is_empty() {
            let trimmed = current.trim().to_string();
            if trimmed.len() > 2 && !entities.contains(&trimmed) {
                entities.push(trimmed);
            }
        }

        entities
    }

    /// Build a full intel report from a set of ingested facts.
    pub fn build_report(&self, target: Target, facts: Vec<Fact>) -> IntelReport {
        let mut sorted = facts.clone();
        sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal));

        let mut by_tier: HashMap<String, Vec<Fact>> = HashMap::new();
        for fact in &sorted {
            by_tier.entry(format!("{:?}", fact.tier))
                .or_insert_with(Vec::new)
                .push(fact.clone());
        }

        let mut timeline: Vec<(String, Fact)> = sorted.iter()
            .filter_map(|f| f.date.as_ref().map(|d| (d.clone(), f.clone())))
            .collect();
        timeline.sort_by(|a, b| a.0.cmp(&b.0));

        let mut related_entities: Vec<String> = Vec::new();
        for fact in &sorted {
            for entity in &fact.entities {
                if !related_entities.contains(entity) && entity != &target.name {
                    related_entities.push(entity.clone());
                }
            }
        }

        let mut source_set: HashMap<String, f64> = HashMap::new();
        for fact in &sorted {
            let trust = fact.source_category.base_trust();
            source_set.entry(fact.source.clone())
                .and_modify(|t| *t = t.max(trust))
                .or_insert(trust);
        }
        let mut sources: Vec<(String, f64)> = source_set.into_iter().collect();
        sources.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let summary = Self::generate_summary(&target, &sorted);

        IntelReport {
            target,
            facts: sorted,
            by_tier,
            timeline,
            related_entities,
            sources,
            summary,
            queries_run: 0, // Filled by caller
        }
    }

    fn generate_summary(target: &Target, facts: &[Fact]) -> String {
        if facts.is_empty() {
            return format!("No confirmed information gathered about {}.", target.name);
        }
        let high_conf_count = facts.iter().filter(|f| f.confidence > 0.7).count();
        let source_count: std::collections::HashSet<&str> = facts.iter()
            .map(|f| f.source.as_str()).collect();

        format!(
            "Gathered {} facts about {} ({} high-confidence) from {} sources. Top finding: {}",
            facts.len(),
            target.name,
            high_conf_count,
            source_count.len(),
            crate::truncate_str(&facts[0].statement, 120),
        )
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_classification_person() {
        let t = Target::new("William Armstrong");
        assert_eq!(t.target_type, TargetType::Person);
    }

    #[test]
    fn test_target_classification_org() {
        let t = Target::new("Gulf Gas Station");
        assert_eq!(t.target_type, TargetType::Organization);
    }

    #[test]
    fn test_query_plan_covers_multiple_sources() {
        let target = Target::new("William Armstrong").with_type(TargetType::Person);
        let queries = QueryPlanner::plan(&target);

        assert!(queries.len() >= 5, "Should have 5+ queries for a person");

        // Should query multiple source categories.
        let all_cats: std::collections::HashSet<String> = queries.iter()
            .flat_map(|q| q.source_types.iter())
            .map(|c| format!("{:?}", c))
            .collect();
        assert!(all_cats.len() >= 3, "Should cover 3+ source categories");

        // Queries sorted by priority.
        for i in 1..queries.len() {
            assert!(queries[i - 1].priority >= queries[i].priority);
        }
    }

    #[test]
    fn test_focus_adds_queries() {
        let base = Target::new("Apache httpd");
        let base_queries = QueryPlanner::plan(&base);

        let sec = Target::new("Apache httpd").with_focus(QueryFocus::Security);
        let sec_queries = QueryPlanner::plan(&sec);

        assert!(sec_queries.len() > base_queries.len(),
            "Security focus should add queries");
        assert!(sec_queries.iter().any(|q| q.query_text.contains("CVE")));
    }

    #[test]
    fn test_context_disambiguation() {
        let target = Target::new("William Armstrong")
            .with_context("British engineer");
        let queries = QueryPlanner::plan(&target);

        assert!(queries.iter().any(|q|
            q.query_text.contains("British engineer")
        ), "Context should appear in some query");
    }

    #[test]
    fn test_entity_extraction() {
        let text = "Barack Obama was born in Honolulu, Hawaii in 1961.";
        let entities = InfoRetrievalEngine::extract_entities(text);
        assert!(entities.iter().any(|e| e.contains("Obama")));
    }

    #[test]
    fn test_fact_ingestion() {
        let mut engine = InfoRetrievalEngine::new();
        let fact = engine.ingest_fact(
            "William Armstrong invented the hydraulic crane in 1846",
            "wikipedia",
            Some("1846"),
        );
        assert!(!fact.statement.is_empty());
        assert_eq!(fact.source, "wikipedia");
        assert!(fact.confidence > 0.0);
    }

    #[test]
    fn test_report_builds_from_facts() {
        let mut engine = InfoRetrievalEngine::new();
        let target = Target::new("Test Entity").with_type(TargetType::Organization);

        let facts = vec![
            engine.ingest_fact("Test Entity was founded in 1900.", "wikipedia", Some("1900")),
            engine.ingest_fact("Test Entity has 500 employees.", "sec_filings", None),
            engine.ingest_fact("Test Entity was acquired by Parent Corp in 2020.", "news_api", Some("2020")),
        ];

        let report = engine.build_report(target, facts);

        assert_eq!(report.facts.len(), 3);
        assert!(!report.summary.is_empty());
        assert_eq!(report.timeline.len(), 2, "Should have 2 dated facts");
    }

    #[test]
    fn test_report_formatting() {
        let mut engine = InfoRetrievalEngine::new();
        let target = Target::new("Acme Corp").with_type(TargetType::Organization);
        let fact = engine.ingest_fact("Acme Corp makes widgets.", "wikipedia", None);
        let report = engine.build_report(target, vec![fact]);

        let formatted = report.format();
        assert!(formatted.contains("Intel Report: Acme Corp"));
        assert!(formatted.contains("Acme Corp makes widgets"));
    }

    #[test]
    fn test_priority_ordering() {
        let target = Target::new("William Armstrong").with_type(TargetType::Person);
        let queries = QueryPlanner::plan(&target);

        // Most important query is the direct name lookup.
        assert!(queries[0].query_text == "William Armstrong"
            || queries[0].priority >= 0.9);
    }

    #[test]
    fn test_security_focus_queries() {
        let t = Target::new("nginx").with_focus(QueryFocus::Security);
        let queries = QueryPlanner::plan(&t);

        let has_cve = queries.iter().any(|q|
            q.query_text.to_lowercase().contains("cve")
            || q.query_text.to_lowercase().contains("vulnerability"));
        assert!(has_cve, "Security focus should generate CVE/vuln queries");
    }

    #[test]
    fn test_technical_focus_queries() {
        let t = Target::new("Transformer").with_focus(QueryFocus::Technical);
        let queries = QueryPlanner::plan(&t);

        let has_research = queries.iter().any(|q|
            q.query_text.contains("research") || q.query_text.contains("paper"));
        assert!(has_research, "Technical focus should find research queries");
    }

    // ============================================================
    // Stress / invariant tests for InfoRetrievalEngine
    // ============================================================

    /// INVARIANT: `plan` always produces at least one query — the raw name
    /// query is unconditional. Empty plans mean the caller can't make
    /// progress, which is a design failure.
    #[test]
    fn invariant_plan_always_includes_raw_name_query() {
        let types = [
            TargetType::Person,
            TargetType::Organization,
            TargetType::Event,
            TargetType::Location,
            TargetType::Product,
            TargetType::Topic,
            TargetType::Unknown,
        ];
        for name in ["Alice", "Acme Corp", "CVE-2025-12345", "x"] {
            for tt in &types {
                let t = Target::new(name).with_type(tt.clone());
                let queries = QueryPlanner::plan(&t);
                assert!(!queries.is_empty(),
                    "plan must not be empty for target '{}' type {:?}", name, tt);
                assert!(queries.iter().any(|q| q.query_text == name),
                    "plan must include the raw name query for '{}'", name);
            }
        }
    }

    /// INVARIANT: every planned query carries a positive priority and a
    /// non-empty rationale. An unprioritised query is a latent bug — the
    /// planner reports confidence that doesn't exist.
    #[test]
    fn invariant_plan_priorities_and_rationales_well_formed() {
        let targets = [
            Target::new("Linux kernel").with_type(TargetType::Product),
            Target::new("2025 election").with_type(TargetType::Event),
            Target::new("San Francisco").with_type(TargetType::Location),
            Target::new("Alan Turing").with_type(TargetType::Person),
        ];
        for t in &targets {
            for q in QueryPlanner::plan(t) {
                assert!(q.priority > 0.0 && q.priority.is_finite(),
                    "query priority must be finite + positive: {}", q.priority);
                assert!(!q.rationale.is_empty(),
                    "every query needs a rationale");
                assert!(!q.source_types.is_empty(),
                    "every query must target at least one source category");
            }
        }
    }

    /// INVARIANT: `ingest_fact` always produces a Fact with finite
    /// confidence in [0, 1] and a non-empty source/statement.
    #[test]
    fn invariant_ingest_fact_returns_well_formed_fact() {
        let mut eng = InfoRetrievalEngine::new();
        let sources = ["wikipedia", "news_api", "arxiv", "social_media", "unknown"];
        for src in sources {
            let fact = eng.ingest_fact("Alice founded Acme.", src, Some("2024-01-01"));
            assert!(fact.confidence >= 0.0 && fact.confidence <= 1.0 && fact.confidence.is_finite(),
                "confidence out of [0,1] or non-finite for source {}: {}", src, fact.confidence);
            assert!(!fact.statement.is_empty());
            assert!(!fact.source.is_empty());
            assert_eq!(fact.date.as_deref(), Some("2024-01-01"));
        }
    }

    /// INVARIANT: entity extraction never panics on arbitrary unicode or
    /// pathological inputs, and every returned entity is non-empty.
    #[test]
    fn invariant_extract_entities_safe_on_arbitrary_input() {
        let weird_inputs = [
            "",
            "   ",
            "ALLCAPS ALLCAPS ALLCAPS",
            "lowercase only words here",
            "Mixed Case With Émoji 🦀 and ñoñ-ascii",
            "Numbers123 456 789Mixed",
            "A B C D E F",
            "X",
            "アリス visited パリ",
        ];
        for input in weird_inputs {
            let entities = InfoRetrievalEngine::extract_entities(input);
            for e in &entities {
                assert!(!e.is_empty(),
                    "extracted entity must not be empty, from input {:?}", input);
                assert!(e.trim() == e,
                    "extracted entity must have no leading/trailing whitespace: {:?}", e);
            }
        }
    }

    /// INVARIANT: `build_report` preserves every input fact. Dropping a
    /// fact silently would corrupt an analyst's understanding.
    #[test]
    fn invariant_build_report_preserves_all_facts() {
        let eng = InfoRetrievalEngine::new();
        let target = Target::new("Subject").with_type(TargetType::Person);
        let facts = vec![
            Fact {
                statement: "A".into(), source: "wikipedia".into(),
                source_category: SourceCategory::Community,
                date: None, entities: vec![],
                tier: crate::intelligence::epistemic_filter::KnowledgeTier::Plausible,
                confidence: 0.6,
            },
            Fact {
                statement: "B".into(), source: "arxiv".into(),
                source_category: SourceCategory::PeerReviewed,
                date: Some("2024".into()), entities: vec![],
                tier: crate::intelligence::epistemic_filter::KnowledgeTier::Corroborated,
                confidence: 0.9,
            },
            Fact {
                statement: "C".into(), source: "social_media".into(),
                source_category: SourceCategory::Anonymous,
                date: None, entities: vec![],
                tier: crate::intelligence::epistemic_filter::KnowledgeTier::Suspect,
                confidence: 0.1,
            },
        ];
        let report = eng.build_report(target, facts.clone());
        // Every fact should reach the report — either via the main facts list
        // or via the by-tier groupings. The sum across tiers should equal 3.
        let total_by_tier: usize = report.by_tier.values().map(|v| v.len()).sum();
        assert_eq!(total_by_tier, 3,
            "build_report must keep all 3 facts in by_tier groups");
    }

    /// INVARIANT: planning for the same target twice is deterministic.
    /// An analyst re-running a case should see identical queries.
    #[test]
    fn invariant_plan_is_deterministic() {
        let t = Target::new("TestTarget")
            .with_type(TargetType::Organization)
            .with_focus(QueryFocus::Financial)
            .with_context("context disambiguator");
        let q1 = QueryPlanner::plan(&t);
        let q2 = QueryPlanner::plan(&t);
        assert_eq!(q1.len(), q2.len(),
            "plan must return the same number of queries each time");
        for (a, b) in q1.iter().zip(q2.iter()) {
            assert_eq!(a.query_text, b.query_text);
            assert_eq!(a.priority, b.priority);
            assert_eq!(a.rationale, b.rationale);
        }
    }

    /// INVARIANT: plan always returns non-empty query list for any target.
    #[test]
    fn invariant_plan_always_non_empty() {
        let targets = [
            Target::new("Basic"),
            Target::new("Complex").with_type(TargetType::Person).with_focus(QueryFocus::Technical),
            Target::new("αβγ").with_context("unicode target"),
        ];
        for t in targets {
            let queries = QueryPlanner::plan(&t);
            assert!(!queries.is_empty(),
                "plan should return non-empty queries for {:?}", t.name);
        }
    }

    /// INVARIANT: Target builder is chainable.
    #[test]
    fn invariant_target_builder_chainable() {
        let t = Target::new("Test")
            .with_type(TargetType::Organization)
            .with_focus(QueryFocus::Financial)
            .with_context("ctx");
        assert_eq!(t.name, "Test");
        assert_eq!(t.target_type, TargetType::Organization);
    }
}
