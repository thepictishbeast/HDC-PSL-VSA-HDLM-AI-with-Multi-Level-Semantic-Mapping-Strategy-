// ============================================================
// LFI Background Learning Daemon — Continuous Self-Improvement
//
// Runs in a separate thread, continuously:
// 1. Searches the web for topics in its research queue
// 2. Cross-references findings against multiple sources
// 3. Ingests verified knowledge into persistent store
// 4. Examines its own source code for self-improvement
// 5. Discovers new topics from related concepts
//
// Toggleable: can be turned on/off to manage power/performance.
// Uses a shared state protected by Arc<Mutex<>> for thread safety.
// ============================================================

use crate::hdc::error::HdcError;
use crate::intelligence::web_search::{WebSearchEngine, SearchResponse};
use crate::intelligence::persistence::{KnowledgeStore, StoredConcept};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use parking_lot::Mutex;
use std::time::Duration;

/// Shared state between the background learner and the main agent.
pub struct SharedKnowledge {
    /// The persistent knowledge store.
    pub store: KnowledgeStore,
    /// Queue of topics to research.
    pub research_queue: Vec<String>,
    /// Recently learned items (for the main agent to incorporate).
    pub recent_learnings: Vec<RecentLearning>,
}

/// A recently learned item from background research.
#[derive(Debug, Clone)]
pub struct RecentLearning {
    /// The topic that was researched.
    pub topic: String,
    /// Summary of what was learned.
    pub summary: String,
    /// Trust score from cross-referencing.
    pub trust: f64,
    /// Number of sources that agreed.
    pub source_count: usize,
}

/// The background learning daemon.
pub struct BackgroundLearner {
    /// Whether the daemon is currently running.
    running: Arc<AtomicBool>,
    /// Shared knowledge state.
    shared: Arc<Mutex<SharedKnowledge>>,
    /// Handle to the background thread.
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl BackgroundLearner {
    /// Create a new background learner with shared state.
    pub fn new(store: KnowledgeStore) -> Self {
        debuglog!("BackgroundLearner::new: creating daemon");
        let shared = Arc::new(Mutex::new(SharedKnowledge {
            store,
            research_queue: Vec::new(),
            recent_learnings: Vec::new(),
        }));

        Self {
            running: Arc::new(AtomicBool::new(false)),
            shared,
            thread_handle: None,
        }
    }

    /// Get a reference to the shared knowledge for the main agent.
    pub fn shared_knowledge(&self) -> Arc<Mutex<SharedKnowledge>> {
        Arc::clone(&self.shared)
    }

    /// Check if the daemon is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the background learning daemon.
    pub fn start(&mut self) -> Result<(), HdcError> {
        if self.is_running() {
            debuglog!("BackgroundLearner::start: already running");
            return Ok(());
        }

        debuglog!("BackgroundLearner::start: ACTIVATING background learning daemon");
        self.running.store(true, Ordering::Relaxed);

        let running = Arc::clone(&self.running);
        let shared = Arc::clone(&self.shared);

        let handle = std::thread::spawn(move || {
            Self::learning_loop(running, shared);
        });

        self.thread_handle = Some(handle);
        debuglog!("BackgroundLearner::start: daemon thread spawned");
        Ok(())
    }

    /// Stop the background learning daemon.
    pub fn stop(&mut self) -> Result<(), HdcError> {
        debuglog!("BackgroundLearner::stop: DEACTIVATING background learning daemon");
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.thread_handle.take() {
            // Don't block forever waiting for the thread
            debuglog!("BackgroundLearner::stop: waiting for daemon thread to finish");
            let _ = handle.join();
        }

        // Save knowledge to disk
        let store_path = KnowledgeStore::default_path();
        let mut guard = self.shared.lock();
        if let Err(e) = guard.store.save(&store_path) {
            debuglog!("BackgroundLearner::stop: failed to save knowledge: {:?}", e);
        }

        debuglog!("BackgroundLearner::stop: daemon stopped and knowledge saved");
        Ok(())
    }

    /// Add a topic to the research queue.
    pub fn enqueue_research(&self, topic: &str) {
        debuglog!("BackgroundLearner::enqueue_research: '{}'", topic);
        let mut guard = self.shared.lock();
        if !guard.research_queue.contains(&topic.to_string()) &&
           !guard.store.has_searched(topic) {
            guard.research_queue.push(topic.to_string());
        }
    }

    /// Drain recent learnings (main agent consumes these).
    pub fn drain_recent_learnings(&self) -> Vec<RecentLearning> {
        let mut guard = self.shared.lock();
        std::mem::take(&mut guard.recent_learnings)
    }

    /// The main learning loop (runs in background thread).
    fn learning_loop(running: Arc<AtomicBool>, shared: Arc<Mutex<SharedKnowledge>>) {
        debuglog!("BackgroundLearner::learning_loop: STARTED");
        let search_engine = WebSearchEngine::new();

        // Seed initial research topics from unknown concepts
        {
            let guard = shared.lock();
            let topics: Vec<String> = guard.store.concepts.iter()
                .filter(|c| c.mastery < 0.5)
                .map(|c| c.name.replace('_', " "))
                .take(10)
                .collect();
            drop(guard);

            let mut guard = shared.lock();
            for topic in topics {
                if !guard.research_queue.contains(&topic) && !guard.store.has_searched(&topic) {
                    guard.research_queue.push(topic);
                }
            }
        }

        let mut cycle_count = 0_u64;

        while running.load(Ordering::Relaxed) {
            cycle_count += 1;
            debuglog!("BackgroundLearner::learning_loop: cycle #{}", cycle_count);

            // Get next topic from queue
            let topic = {
                let mut guard = shared.lock();
                guard.research_queue.pop()
            };

            if let Some(topic) = topic {
                debuglog!("BackgroundLearner::learning_loop: researching '{}'", topic);

                // Search the web
                match search_engine.search(&topic) {
                    Ok(response) => {
                        Self::process_search_results(&shared, &topic, &response);
                    }
                    Err(e) => {
                        debuglog!("BackgroundLearner::learning_loop: search failed for '{}': {:?}", topic, e);
                    }
                }

                // Mark as searched
                {
                    let mut guard = shared.lock();
                    guard.store.mark_searched(&topic);
                }

                // Rate limit: wait between searches
                std::thread::sleep(Duration::from_secs(30));
            } else {
                // No topics in queue — perform self-examination or discover new topics
                if cycle_count % 5 == 0 {
                    Self::perform_self_examination(&shared);
                } else {
                    Self::discover_new_topics(&shared);
                }

                // Longer sleep when idle
                std::thread::sleep(Duration::from_secs(60));
            }

            // Periodically save to disk
            if cycle_count % 10 == 0 {
                let store_path = KnowledgeStore::default_path();
                let mut guard = shared.lock();
                if let Err(e) = guard.store.save(&store_path) {
                    debuglog!("BackgroundLearner::learning_loop: periodic save failed: {:?}", e);
                }
            }
        }

        debuglog!("BackgroundLearner::learning_loop: STOPPED after {} cycles", cycle_count);
    }

    /// Examine the project's own source code to learn about its architecture.
    fn perform_self_examination(shared: &Arc<Mutex<SharedKnowledge>>) {
        debuglog!("BackgroundLearner::perform_self_examination: Analyzing own source code");
        
        // Target key source files
        let files = [
            "src/lib.rs",
            "src/agent.rs",
            "src/cognition/reasoner.rs",
            "src/cognition/knowledge.rs",
            "src/hdc/vector.rs",
        ];

        let mut findings = Vec::new();
        for file_path in &files {
            let full_path = format!("/root/lfi_project/lfi_vsa_core/{}", file_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                // Naive extraction of key technical terms (structs, enums, etc.)
                let lines: Vec<&str> = content.lines().collect();
                for line in lines {
                    let line = line.trim();
                    if line.starts_with("pub struct") || line.starts_with("pub enum") || line.starts_with("pub trait") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            findings.push(parts[2].trim_end_matches('{').trim().to_string());
                        }
                    }
                }
            }
        }

        if !findings.is_empty() {
            let mut guard = shared.lock();
            for finding in findings {
                let concept_key = format!("internal_{}", finding.to_lowercase());
                if !guard.store.concepts.iter().any(|c| c.name == concept_key) {
                    debuglog!("BackgroundLearner: Discovered internal structure: {}", finding);
                    guard.store.upsert_concept(StoredConcept {
                        name: concept_key.clone(),
                        mastery: 0.9, // High mastery for self-source
                        encounter_count: 1,
                        trust_score: 1.0, // Absolute trust in source
                        related_concepts: vec!["architecture".to_string(), "vsa_internal".to_string()],
                        definition: Some(format!("Internal component of the Sovereign Intelligence identified during self-examination.")),
                        first_learned: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        last_reinforced: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    });
                }
            }
        }
    }

    /// Process search results and ingest verified knowledge.
    fn process_search_results(
        shared: &Arc<Mutex<SharedKnowledge>>,
        topic: &str,
        response: &SearchResponse,
    ) {
        debuglog!(
            "BackgroundLearner::process_search_results: topic='{}', {} results, trust={:.2}",
            topic, response.results.len(), response.cross_reference_trust
        );

        if response.results.is_empty() {
            debuglog!("BackgroundLearner::process_search_results: no results for '{}'", topic);
            return;
        }

        // Only ingest if cross-reference trust is above threshold
        let min_trust = 0.3; // At least 1 source needed
        if response.cross_reference_trust < min_trust {
            debuglog!(
                "BackgroundLearner::process_search_results: trust too low ({:.2} < {:.2})",
                response.cross_reference_trust, min_trust
            );
            return;
        }

        let summary = &response.best_summary;
        if summary.is_empty() {
            return;
        }

        // Extract related concepts from the summary
        let related: Vec<String> = summary.split_whitespace()
            .filter(|w: &&str| w.len() > 4)
            .take(10)
            .map(|w: &str| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w: &String| !w.is_empty())
            .collect();

        let concept_key = topic.to_lowercase().replace(' ', "_");

        let mut guard = shared.lock();

        // Store the concept
        guard.store.upsert_concept(StoredConcept {
            name: concept_key.clone(),
            mastery: (response.cross_reference_trust * 0.5).min(0.7), // Cap at 0.7 for web-learned
            encounter_count: response.results.len(),
            trust_score: response.cross_reference_trust,
            related_concepts: related,
            definition: Some(summary[..summary.len().min(500)].to_string()),
            first_learned: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            last_reinforced: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        });

        // Also store as a fact for quick retrieval
        guard.store.upsert_fact(&concept_key, &summary[..summary.len().min(500)]);

        // Add to recent learnings for the main agent
        guard.recent_learnings.push(RecentLearning {
            topic: topic.to_string(),
            summary: summary[..summary.len().min(200)].to_string(),
            trust: response.cross_reference_trust,
            source_count: response.source_count,
        });

        guard.store.log_learning(&format!(
            "Learned '{}' from {} sources (trust={:.2}): {}",
            topic, response.source_count, response.cross_reference_trust,
            &summary[..summary.len().min(100)]
        ));

        debuglog!(
            "BackgroundLearner::process_search_results: ingested '{}' (trust={:.2})",
            topic, response.cross_reference_trust
        );
    }

    /// Discover new topics to research from existing knowledge gaps.
    fn discover_new_topics(shared: &Arc<Mutex<SharedKnowledge>>) {
        let mut guard = shared.lock();

        // Find concepts with low mastery and unsearched related concepts
        let mut new_topics = Vec::new();
        for concept in &guard.store.concepts {
            if concept.mastery < 0.6 {
                for related in &concept.related_concepts {
                    let topic = related.replace('_', " ");
                    if !guard.store.has_searched(&topic) &&
                       !guard.research_queue.contains(&topic) &&
                       !new_topics.contains(&topic) {
                        new_topics.push(topic);
                    }
                }
            }
        }

        // Add top 5 new topics
        for topic in new_topics.into_iter().take(5) {
            debuglog!("BackgroundLearner::discover_new_topics: queuing '{}'", topic);
            guard.research_queue.push(topic);
        }
    }
}

impl Drop for BackgroundLearner {
    fn drop(&mut self) {
        if self.is_running() {
            debuglog!("BackgroundLearner::drop: stopping daemon on drop");
            self.running.store(false, Ordering::Relaxed);
            // Don't wait for the thread — it will stop on next cycle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_learner_creation() {
        let store = KnowledgeStore::new();
        let learner = BackgroundLearner::new(store);
        assert!(!learner.is_running());
    }

    #[test]
    fn test_enqueue_research() {
        let store = KnowledgeStore::new();
        let learner = BackgroundLearner::new(store);
        learner.enqueue_research("quantum computing");
        learner.enqueue_research("quantum computing"); // Duplicate should be ignored

        let guard = learner.shared_knowledge();
        let locked = guard.lock();
        assert_eq!(locked.research_queue.len(), 1);
    }

    #[test]
    fn test_drain_recent_learnings() {
        let store = KnowledgeStore::new();
        let learner = BackgroundLearner::new(store);

        // Manually add a learning
        {
            let guard = learner.shared_knowledge();
            let mut locked = guard.lock();
            locked.recent_learnings.push(RecentLearning {
                topic: "test".to_string(),
                summary: "A test summary".to_string(),
                trust: 0.8,
                source_count: 2,
            });
        }

        let learnings = learner.drain_recent_learnings();
        assert_eq!(learnings.len(), 1);
        assert_eq!(learnings[0].topic, "test");

        // Should be empty now
        let learnings2 = learner.drain_recent_learnings();
        assert!(learnings2.is_empty());
    }
}
