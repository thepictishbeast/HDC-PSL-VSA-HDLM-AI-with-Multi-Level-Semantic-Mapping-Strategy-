//! # Purpose
//! Experience-based learning — captures learning signals from every user
//! interaction and feeds them back into the knowledge base immediately.
//! This closes the loop between using the system and training it.
//! The more the system is used, the smarter it gets.
//!
//! # Design Decisions
//! - Signals captured: corrections, follow-ups, regenerations, knowledge gaps,
//!   tool failures, positive feedback (thumbs up)
//! - Each signal type produces a different training action
//! - Signals are persisted to brain.db for batch training AND applied immediately
//!   to the agent's in-memory knowledge
//! - No signal is thrown away — even "the user moved on" is informative
//!
//! # Invariants
//! - Signal capture must NEVER slow down the response path (async write-behind)
//! - All signals are attributed to a conversation_id for context
//! - Corrections always produce both a negative example AND a corrected fact
//!
//! # Failure Modes
//! - If brain.db is locked, signals queue in memory and flush on next opportunity
//! - If the correction is itself wrong, the system may learn incorrect information
//!   (mitigated by PSL axiom validation on the corrected fact)

use std::collections::VecDeque;

/// A learning signal captured from user interaction.
#[derive(Debug, Clone)]
pub struct LearningSignal {
    /// What type of signal this is.
    pub signal_type: SignalType,
    /// The original user input that triggered the signal.
    pub user_input: String,
    /// The system's response that the signal is about.
    pub system_response: String,
    /// The correction or follow-up content (if applicable).
    pub correction: Option<String>,
    /// Conversation context ID.
    pub conversation_id: Option<String>,
    /// When this signal was captured.
    pub timestamp: u64,
}

/// Types of learning signals from user interactions.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    /// User explicitly corrected the system ("that's wrong", "no, it's X")
    Correction,
    /// User asked a follow-up, implying the answer was incomplete
    FollowUp,
    /// User regenerated the response (thumbs down / retry)
    Regeneration,
    /// System couldn't answer — knowledge gap detected
    KnowledgeGap,
    /// User gave positive feedback (thumbs up, "thanks", "perfect")
    PositiveFeedback,
    /// User asked about a topic with zero relevant facts in brain.db
    ZeroCoverage,
    /// Web search was triggered — system lacked the knowledge internally
    WebSearchFallback,
}

/// The experience learning engine.
pub struct ExperienceLearner {
    /// Pending signals that haven't been persisted yet.
    pending: VecDeque<LearningSignal>,
    /// Maximum pending signals before force-flush.
    max_pending: usize,
    /// Statistics for monitoring.
    pub stats: ExperienceStats,
}

/// Statistics about learning from experience.
#[derive(Debug, Clone, Default)]
pub struct ExperienceStats {
    pub corrections_captured: u64,
    pub follow_ups_captured: u64,
    pub regenerations_captured: u64,
    pub knowledge_gaps_detected: u64,
    pub positive_feedback: u64,
    pub facts_created_from_signals: u64,
    pub adversarial_examples_from_corrections: u64,
}

impl ExperienceLearner {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            max_pending: 100,
            stats: ExperienceStats::default(),
        }
    }

    /// Capture a learning signal from user interaction.
    pub fn capture(&mut self, signal: LearningSignal) {
        match signal.signal_type {
            SignalType::Correction => self.stats.corrections_captured += 1,
            SignalType::FollowUp => self.stats.follow_ups_captured += 1,
            SignalType::Regeneration => self.stats.regenerations_captured += 1,
            SignalType::KnowledgeGap | SignalType::ZeroCoverage => {
                self.stats.knowledge_gaps_detected += 1;
            }
            SignalType::PositiveFeedback => self.stats.positive_feedback += 1,
            SignalType::WebSearchFallback => self.stats.knowledge_gaps_detected += 1,
        }
        self.pending.push_back(signal);
    }

    /// Process pending signals and return training actions.
    /// Each action describes what should be done with the signal:
    /// - Create a new fact in brain.db
    /// - Create an adversarial example
    /// - Flag a domain for more ingestion
    /// - Reinforce an existing fact
    pub fn process_pending(&mut self) -> Vec<TrainingAction> {
        let mut actions = Vec::new();

        while let Some(signal) = self.pending.pop_front() {
            match signal.signal_type {
                SignalType::Correction => {
                    // Create adversarial example from the wrong answer
                    actions.push(TrainingAction::CreateAdversarial {
                        claim: signal.system_response.clone(),
                        label: "refuted".into(),
                        explanation: format!(
                            "User corrected this response. Original input: '{}'",
                            truncate(&signal.user_input, 100)
                        ),
                    });
                    // Downgrade quality of the original wrong response in brain.db
                    // AUDIT FIX #19: Quality scores now update from user feedback
                    if !signal.system_response.is_empty() {
                        actions.push(TrainingAction::DowngradeQuality {
                            content_fragment: truncate(&signal.system_response, 200).to_string(),
                            new_quality: 0.3, // Corrected facts get low quality
                        });
                    }
                    // Create corrected fact if correction provided
                    if let Some(ref correction) = signal.correction {
                        actions.push(TrainingAction::CreateFact {
                            key: format!("correction_{}", signal.timestamp),
                            value: correction.clone(),
                            source: "user_correction".into(),
                            domain: "corrected".into(),
                            confidence: 0.95, // User corrections are high-confidence
                        });
                        self.stats.facts_created_from_signals += 1;
                    }
                    self.stats.adversarial_examples_from_corrections += 1;
                }
                SignalType::FollowUp => {
                    // Flag the topic as needing more depth
                    actions.push(TrainingAction::FlagForDepth {
                        topic: signal.user_input.clone(),
                        reason: "User asked follow-up, indicating incomplete initial answer".into(),
                    });
                }
                SignalType::Regeneration => {
                    // The original response was unsatisfactory — negative example
                    actions.push(TrainingAction::CreateAdversarial {
                        claim: signal.system_response.clone(),
                        label: "refuted".into(),
                        explanation: format!(
                            "User regenerated this response (unsatisfactory). Input: '{}'",
                            truncate(&signal.user_input, 100)
                        ),
                    });
                    self.stats.adversarial_examples_from_corrections += 1;
                }
                SignalType::KnowledgeGap | SignalType::ZeroCoverage => {
                    // Queue the domain for more ingestion
                    actions.push(TrainingAction::FlagForIngestion {
                        query: signal.user_input.clone(),
                        reason: "No relevant facts found for this query".into(),
                    });
                }
                SignalType::PositiveFeedback => {
                    // Reinforce the knowledge that produced the good answer
                    actions.push(TrainingAction::Reinforce {
                        query: signal.user_input.clone(),
                        response: signal.system_response.clone(),
                    });
                }
                SignalType::WebSearchFallback => {
                    actions.push(TrainingAction::FlagForIngestion {
                        query: signal.user_input.clone(),
                        reason: "Web search was needed — internal knowledge insufficient".into(),
                    });
                }
            }
        }

        actions
    }

    /// Number of pending signals.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// An action to take based on a learning signal.
#[derive(Debug, Clone)]
pub enum TrainingAction {
    /// Create a new fact in brain.db.
    CreateFact {
        key: String,
        value: String,
        source: String,
        domain: String,
        confidence: f64,
    },
    /// Create an adversarial example (negative training signal).
    CreateAdversarial {
        claim: String,
        label: String,
        explanation: String,
    },
    /// Flag a topic as needing more depth in the knowledge base.
    FlagForDepth {
        topic: String,
        reason: String,
    },
    /// Flag a query for more data ingestion.
    FlagForIngestion {
        query: String,
        reason: String,
    },
    /// Reinforce existing knowledge (boost confidence/mastery).
    Reinforce {
        query: String,
        response: String,
    },
    /// Downgrade quality of facts matching a content fragment.
    /// AUDIT FIX #19: Corrections now feed back into quality scoring.
    DowngradeQuality {
        content_fragment: String,
        new_quality: f64,
    },
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(st: SignalType) -> LearningSignal {
        LearningSignal {
            signal_type: st,
            user_input: "test input".into(),
            system_response: "test response".into(),
            correction: Some("corrected answer".into()),
            conversation_id: Some("conv_1".into()),
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_capture_correction() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::Correction));
        assert_eq!(learner.stats.corrections_captured, 1);
        assert_eq!(learner.pending_count(), 1);
    }

    #[test]
    fn test_process_correction_produces_adversarial_and_fact() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::Correction));
        let actions = learner.process_pending();
        // AUDIT FIX #19 added DowngradeQuality between adversarial and fact
        assert_eq!(actions.len(), 3); // adversarial + downgrade + corrected fact
        assert!(matches!(actions[0], TrainingAction::CreateAdversarial { .. }));
        assert!(matches!(actions[1], TrainingAction::DowngradeQuality { .. }));
        assert!(matches!(actions[2], TrainingAction::CreateFact { .. }));
    }

    #[test]
    fn test_knowledge_gap_flags_ingestion() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::KnowledgeGap));
        let actions = learner.process_pending();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], TrainingAction::FlagForIngestion { .. }));
    }

    #[test]
    fn test_positive_feedback_reinforces() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::PositiveFeedback));
        let actions = learner.process_pending();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], TrainingAction::Reinforce { .. }));
    }

    #[test]
    fn test_regeneration_creates_adversarial() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::Regeneration));
        let actions = learner.process_pending();
        assert!(matches!(actions[0], TrainingAction::CreateAdversarial { .. }));
    }

    #[test]
    fn test_stats_accumulate() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::Correction));
        learner.capture(make_signal(SignalType::FollowUp));
        learner.capture(make_signal(SignalType::KnowledgeGap));
        assert_eq!(learner.stats.corrections_captured, 1);
        assert_eq!(learner.stats.follow_ups_captured, 1);
        assert_eq!(learner.stats.knowledge_gaps_detected, 1);
        assert_eq!(learner.pending_count(), 3);
    }

    #[test]
    fn test_process_clears_pending() {
        let mut learner = ExperienceLearner::new();
        learner.capture(make_signal(SignalType::Correction));
        learner.capture(make_signal(SignalType::FollowUp));
        let _actions = learner.process_pending();
        assert_eq!(learner.pending_count(), 0);
    }
}
