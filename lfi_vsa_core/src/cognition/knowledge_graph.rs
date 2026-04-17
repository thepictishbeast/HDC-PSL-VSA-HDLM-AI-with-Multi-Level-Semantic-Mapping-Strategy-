// ============================================================
// Knowledge Graph — Persistent fact connections and domain cross-references
//
// SUPERSOCIETY: With 59M+ facts, linear search is useless.
// This module builds a graph overlay that connects related facts,
// tracks cross-domain concept bridges, and enables graph traversal
// for richer RAG context.
//
// Edge types:
//   related     — shares keywords/concepts
//   supports    — provides evidence for another fact
//   contradicts — conflicts with another fact
//   causal      — cause-effect relationship
//   elaborates  — provides more detail on another fact
//   cross_domain — connects facts across different domains
// ============================================================

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::{info, warn};

use crate::persistence::BrainDb;

/// Edge type taxonomy for the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeType {
    Related,
    Supports,
    Contradicts,
    Causal,
    Elaborates,
    CrossDomain,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Related => "related",
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Causal => "causal",
            Self::Elaborates => "elaborates",
            Self::CrossDomain => "cross_domain",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "supports" => Self::Supports,
            "contradicts" => Self::Contradicts,
            "causal" => Self::Causal,
            "elaborates" => Self::Elaborates,
            "cross_domain" => Self::CrossDomain,
            _ => Self::Related,
        }
    }
}

/// A single edge in the knowledge graph.
#[derive(Debug, Clone)]
pub struct FactEdge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    pub strength: f64,
    pub evidence: Option<String>,
}

/// Result of a graph traversal — a connected subgraph.
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub center: String,
    pub nodes: Vec<String>,
    pub edges: Vec<FactEdge>,
    pub depth_reached: usize,
}

/// Knowledge graph engine backed by persistent storage.
pub struct KnowledgeGraph {
    db: Arc<BrainDb>,
}

impl KnowledgeGraph {
    pub fn new(db: Arc<BrainDb>) -> Self {
        info!("// KNOWLEDGE_GRAPH: initialized");
        Self { db }
    }

    /// Connect two facts with a typed edge.
    /// BUG ASSUMPTION: caller has validated that both fact keys exist.
    pub fn connect(&self, source: &str, target: &str, edge_type: EdgeType, strength: f64, evidence: Option<&str>) {
        self.db.create_edge(source, target, edge_type.as_str(), strength, evidence);
    }

    /// Get all connections for a fact (both directions).
    pub fn connections(&self, fact_key: &str, limit: usize) -> Vec<FactEdge> {
        self.db.get_neighbors(fact_key, limit)
            .into_iter()
            .map(|(neighbor, etype, strength, direction)| {
                let (src, tgt) = if direction == "outbound" {
                    (fact_key.to_string(), neighbor)
                } else {
                    (neighbor, fact_key.to_string())
                };
                FactEdge {
                    source: src,
                    target: tgt,
                    edge_type: EdgeType::from_str(&etype),
                    strength,
                    evidence: None,
                }
            })
            .collect()
    }

    /// BFS traversal from a starting fact, returning connected subgraph up to max_depth.
    /// SECURITY: max_depth capped at 5 to prevent runaway graph traversal on 59M facts.
    pub fn traverse(&self, start: &str, max_depth: usize, max_nodes: usize) -> Subgraph {
        let depth = max_depth.min(5);
        let cap = max_nodes.min(200);

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut all_edges: Vec<FactEdge> = Vec::new();
        let mut max_d = 0;

        visited.insert(start.to_string());
        queue.push_back((start.to_string(), 0));

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth || visited.len() >= cap {
                break;
            }
            max_d = max_d.max(d);

            let neighbors = self.connections(&current, 20);
            for edge in neighbors {
                let neighbor_key = if edge.source == current {
                    &edge.target
                } else {
                    &edge.source
                };

                if !visited.contains(neighbor_key) && visited.len() < cap {
                    visited.insert(neighbor_key.to_string());
                    queue.push_back((neighbor_key.to_string(), d + 1));
                }
                all_edges.push(edge);
            }
        }

        Subgraph {
            center: start.to_string(),
            nodes: visited.into_iter().collect(),
            edges: all_edges,
            depth_reached: max_d,
        }
    }

    /// Find the shortest path between two facts using BFS.
    /// Returns None if no path exists within max_depth hops.
    pub fn shortest_path(&self, from: &str, to: &str, max_depth: usize) -> Option<Vec<String>> {
        let depth = max_depth.min(10);
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

        visited.insert(from.to_string());
        queue.push_back((from.to_string(), vec![from.to_string()]));

        while let Some((current, path)) = queue.pop_front() {
            if current == to {
                return Some(path);
            }
            if path.len() > depth {
                continue;
            }

            let neighbors = self.connections(&current, 50);
            for edge in neighbors {
                let next = if edge.source == current {
                    edge.target.clone()
                } else {
                    edge.source.clone()
                };

                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    let mut new_path = path.clone();
                    new_path.push(next.clone());
                    queue.push_back((next, new_path));
                }
            }
        }
        None
    }

    /// Batch-connect facts that share keywords.
    /// Scans a sample of facts and creates 'related' edges based on shared n-grams.
    /// BUG ASSUMPTION: This is expensive — should run as a background job, not on hot path.
    pub fn build_keyword_edges(&self, sample_size: usize) -> usize {
        let conn = self.db.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Get a random sample of facts with their domains
        let mut stmt = match conn.prepare(
            "SELECT key, value, domain FROM facts WHERE domain IS NOT NULL ORDER BY RANDOM() LIMIT ?1"
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("// KNOWLEDGE_GRAPH: build_keyword_edges query failed: {}", e);
                return 0;
            }
        };

        let facts: Vec<(String, String, String)> = stmt.query_map(
            rusqlite::params![sample_size as i64],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        ).unwrap_or_else(|_| panic!("query_map failed"))
         .filter_map(|r| r.ok())
         .collect();

        drop(stmt);
        drop(conn);

        // Extract significant keywords from each fact
        let stopwords: HashSet<&str> = [
            "the","and","for","are","but","not","you","all","can","had",
            "her","was","one","our","out","has","its","how","who","what",
            "when","where","why","this","that","with","from","they","been",
            "have","many","will","each","make","like","does","into","than",
            "them","then","some","could","other","more","very","just","also",
        ].iter().copied().collect();

        let mut keyword_index: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (key, value, domain) in &facts {
            let keywords: Vec<String> = value.split_whitespace()
                .filter(|w| w.len() >= 4 && !stopwords.contains(w.to_lowercase().as_str()))
                .take(10)
                .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>())
                .filter(|w| w.len() >= 4)
                .collect();

            for kw in keywords {
                keyword_index.entry(kw).or_default().push((key.clone(), domain.clone()));
            }
        }

        // Create edges between facts sharing significant keywords
        let mut edges_created = 0;
        for (keyword, fact_list) in &keyword_index {
            // Skip very common keywords (appearing in >20 facts of sample)
            if fact_list.len() < 2 || fact_list.len() > 20 {
                continue;
            }

            for i in 0..fact_list.len().min(5) {
                for j in (i + 1)..fact_list.len().min(5) {
                    let (ref key_a, ref domain_a) = fact_list[i];
                    let (ref key_b, ref domain_b) = fact_list[j];

                    if key_a == key_b {
                        continue;
                    }

                    let edge_type = if domain_a != domain_b {
                        "cross_domain"
                    } else {
                        "related"
                    };

                    let strength = 0.5 + (0.5 / fact_list.len() as f64);
                    let evidence = format!("shared keyword: {}", keyword);
                    self.db.create_edge(key_a, key_b, edge_type, strength, Some(&evidence));
                    edges_created += 1;
                }
            }
        }

        // Build domain cross-references from the edges we just created
        let mut domain_concepts: HashMap<(String, String), Vec<String>> = HashMap::new();
        for (keyword, fact_list) in &keyword_index {
            if fact_list.len() < 2 || fact_list.len() > 20 {
                continue;
            }
            let domains: HashSet<&String> = fact_list.iter().map(|(_, d)| d).collect();
            let domain_vec: Vec<&String> = domains.into_iter().collect();
            for i in 0..domain_vec.len() {
                for j in (i + 1)..domain_vec.len() {
                    let (da, db) = if domain_vec[i] < domain_vec[j] {
                        (domain_vec[i].clone(), domain_vec[j].clone())
                    } else {
                        (domain_vec[j].clone(), domain_vec[i].clone())
                    };
                    domain_concepts.entry((da, db)).or_default().push(keyword.clone());
                }
            }
        }

        for ((da, db), concepts) in &domain_concepts {
            let top_concept = &concepts[0];
            let strength = (concepts.len() as f64 / 10.0).min(1.0);
            let example_key = keyword_index.get(top_concept)
                .and_then(|list| list.first())
                .map(|(k, _)| k.as_str());
            self.db.create_domain_xref(da, db, top_concept, strength, example_key);
        }

        info!("// KNOWLEDGE_GRAPH: built {} keyword edges, {} domain xrefs from {} facts",
              edges_created, domain_concepts.len(), facts.len());
        edges_created
    }

    /// Get graph statistics for the dashboard.
    pub fn stats(&self) -> GraphStats {
        let edge_count = self.db.count_edges();
        let type_dist = self.db.edge_type_stats();
        let xrefs = self.db.get_all_domain_xrefs();

        GraphStats {
            total_edges: edge_count,
            edge_types: type_dist.into_iter().collect(),
            domain_xref_count: xrefs.len(),
            top_domain_bridges: xrefs.into_iter().take(10)
                .map(|(a, b, concept, strength)| DomainBridge { domain_a: a, domain_b: b, concept, strength })
                .collect(),
        }
    }
}

/// Summary statistics for the knowledge graph.
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_edges: i64,
    pub edge_types: Vec<(String, i64)>,
    pub domain_xref_count: usize,
    pub top_domain_bridges: Vec<DomainBridge>,
}

/// A cross-domain concept bridge.
#[derive(Debug, Clone)]
pub struct DomainBridge {
    pub domain_a: String,
    pub domain_b: String,
    pub concept: String,
    pub strength: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_graph() -> KnowledgeGraph {
        let id = std::process::id();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_kg_{}_{}.db", id, ts));
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(BrainDb::open(&path).unwrap());
        KnowledgeGraph::new(db)
    }

    #[test]
    fn test_connect_and_query() {
        let kg = test_graph();
        kg.connect("fact_a", "fact_b", EdgeType::Related, 0.8, Some("shared topic"));
        kg.connect("fact_a", "fact_c", EdgeType::Supports, 0.9, None);

        let edges = kg.connections("fact_a", 10);
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_bidirectional_query() {
        let kg = test_graph();
        kg.connect("fact_x", "fact_y", EdgeType::Causal, 0.7, Some("x causes y"));

        let from_x = kg.connections("fact_x", 10);
        assert_eq!(from_x.len(), 1);

        let from_y = kg.connections("fact_y", 10);
        assert_eq!(from_y.len(), 1);
    }

    #[test]
    fn test_traverse_bfs() {
        let kg = test_graph();
        // Build a small chain: a -> b -> c -> d
        kg.connect("a", "b", EdgeType::Related, 0.8, None);
        kg.connect("b", "c", EdgeType::Related, 0.7, None);
        kg.connect("c", "d", EdgeType::Related, 0.6, None);

        let subgraph = kg.traverse("a", 3, 100);
        assert!(subgraph.nodes.len() >= 3); // a, b, c at minimum
        assert!(subgraph.nodes.contains(&"a".to_string()));
        assert!(subgraph.nodes.contains(&"b".to_string()));
    }

    #[test]
    fn test_shortest_path() {
        let kg = test_graph();
        kg.connect("start", "mid1", EdgeType::Related, 0.8, None);
        kg.connect("mid1", "mid2", EdgeType::Related, 0.7, None);
        kg.connect("mid2", "end", EdgeType::Related, 0.6, None);

        let path = kg.shortest_path("start", "end", 5);
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.first().unwrap(), "start");
        assert_eq!(p.last().unwrap(), "end");
        assert!(p.len() <= 4);
    }

    #[test]
    fn test_no_path() {
        let kg = test_graph();
        kg.connect("island_a", "island_b", EdgeType::Related, 0.8, None);
        kg.connect("other_x", "other_y", EdgeType::Related, 0.7, None);

        let path = kg.shortest_path("island_a", "other_x", 5);
        assert!(path.is_none());
    }

    #[test]
    fn test_stats() {
        let kg = test_graph();
        kg.connect("f1", "f2", EdgeType::Related, 0.8, None);
        kg.connect("f2", "f3", EdgeType::Contradicts, 0.6, None);
        kg.connect("f3", "f4", EdgeType::CrossDomain, 0.9, None);

        let stats = kg.stats();
        assert_eq!(stats.total_edges, 3);
        assert!(stats.edge_types.len() >= 2);
    }

    #[test]
    fn test_traverse_depth_cap() {
        let kg = test_graph();
        // Deep chain of 20 nodes
        for i in 0..19 {
            kg.connect(&format!("n{}", i), &format!("n{}", i + 1), EdgeType::Related, 0.5, None);
        }

        // Max depth 3 should not reach n19
        let subgraph = kg.traverse("n0", 3, 100);
        assert!(subgraph.nodes.len() <= 5); // n0, n1, n2, n3 max
        assert!(!subgraph.nodes.contains(&"n19".to_string()));
    }

    #[test]
    fn test_edge_type_roundtrip() {
        assert_eq!(EdgeType::from_str("related"), EdgeType::Related);
        assert_eq!(EdgeType::from_str("contradicts"), EdgeType::Contradicts);
        assert_eq!(EdgeType::from_str("causal"), EdgeType::Causal);
        assert_eq!(EdgeType::from_str("cross_domain"), EdgeType::CrossDomain);
        assert_eq!(EdgeType::from_str("unknown_thing"), EdgeType::Related); // default
    }
}
