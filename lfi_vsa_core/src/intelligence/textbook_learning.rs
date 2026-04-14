// ============================================================
// Textbook Learning — Human-Style Study
//
// PURPOSE: Match how humans actually learn:
//   1. Read reference material (textbook chapter, paper, documentation)
//   2. Try to answer questions WITHOUT being told the answer
//   3. Self-grade after answering (check own work vs reference)
//   4. Track what you learned vs what you still don't understand
//   5. Re-read weak sections, try harder questions
//
// THIS IS THE OPPOSITE OF CURRENT SUPERVISED TRAINING:
//   Current: Q + expected_answer → learn
//   Textbook: Q + reference_material → attempt → self-grade
//
// HUMAN LEARNING PROPERTIES THIS REPLICATES:
//   - Primary source reading (textbook is authoritative)
//   - Active recall (answer WITHOUT looking at answer)
//   - Retrieval practice (answer from memory of the reading)
//   - Spaced repetition (weak concepts revisited)
//   - Metacognition (assess own confidence)
//   - Pre-testing (struggle before answer is revealed)
//
// THE "NEVER SEE THE ANSWER" PRINCIPLE:
//   Expected answers are used ONLY for grading. LFI sees:
//     - The textbook material (before)
//     - The question (during)
//     - The grade (after: correct/incorrect)
//   But NEVER:
//     - The expected answer paired with the question (during learning)
//
// This prevents the trivial memorization path (Q→A lookup) and
// forces LFI to actually use the reference material to reason.
// ============================================================

use crate::hdc::error::HdcError;
use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::answer_verifier::AnswerVerifier;
use std::collections::HashMap;

// ============================================================
// Textbook Section
// ============================================================

/// A section of study material.
#[derive(Debug, Clone)]
pub struct TextbookSection {
    pub title: String,
    pub topic: String,
    /// The actual content — paragraphs of explanation.
    pub content: String,
    /// Sub-topics covered.
    pub concepts: Vec<String>,
    /// Difficulty level (0.0 trivial, 1.0 graduate).
    pub difficulty: f64,
}

/// A question attached to a textbook section.
/// BUG ASSUMPTION: expected_answer must never be given to LFI during learning.
/// Only used by the grader after LFI submits an answer.
#[derive(Debug, Clone)]
pub struct TextbookQuestion {
    pub question: String,
    /// Which section this question tests (must read first).
    pub requires_section: String,
    /// The grader's reference answer — HIDDEN from LFI.
    expected_answer: String,
    /// Alternative acceptable answers.
    acceptable_alternatives: Vec<String>,
    pub difficulty: f64,
}

impl TextbookQuestion {
    pub fn new(
        question: &str,
        requires_section: &str,
        expected_answer: &str,
        difficulty: f64,
    ) -> Self {
        Self {
            question: question.into(),
            requires_section: requires_section.into(),
            expected_answer: expected_answer.into(),
            acceptable_alternatives: Vec::new(),
            difficulty,
        }
    }

    pub fn with_alternatives(mut self, alts: Vec<String>) -> Self {
        self.acceptable_alternatives = alts;
        self
    }
}

// ============================================================
// Study Session Result
// ============================================================

#[derive(Debug, Clone)]
pub struct StudyAttempt {
    pub question: String,
    pub section_read: String,
    pub lfi_answer: String,
    pub was_correct: bool,
    pub verification_mode: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct StudySession {
    pub subject: String,
    pub attempts: Vec<StudyAttempt>,
    pub concepts_read: Vec<String>,
    pub correct: usize,
    pub total: usize,
}

impl StudySession {
    pub fn new(subject: &str) -> Self {
        Self {
            subject: subject.into(),
            attempts: Vec::new(),
            concepts_read: Vec::new(),
            correct: 0,
            total: 0,
        }
    }

    pub fn accuracy(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.correct as f64 / self.total as f64 }
    }
}

// ============================================================
// Textbook Learning Engine
// ============================================================

/// Orchestrates textbook-style learning.
pub struct TextbookLearner {
    /// All textbook sections, indexed by title.
    pub sections: HashMap<String, TextbookSection>,
    /// All questions, indexed by required section.
    pub questions: HashMap<String, Vec<TextbookQuestion>>,
    /// Sessions history.
    pub sessions: Vec<StudySession>,
}

impl TextbookLearner {
    pub fn new() -> Self {
        debuglog!("TextbookLearner::new: Initializing human-style learning engine");
        Self {
            sections: HashMap::new(),
            questions: HashMap::new(),
            sessions: Vec::new(),
        }
    }

    /// Add a textbook section.
    pub fn add_section(&mut self, section: TextbookSection) {
        debuglog!("TextbookLearner: Adding section '{}'", section.title);
        self.sections.insert(section.title.clone(), section);
    }

    /// Add questions for a section.
    pub fn add_questions(&mut self, section_title: &str, questions: Vec<TextbookQuestion>) {
        debuglog!("TextbookLearner: Adding {} questions for '{}'",
            questions.len(), section_title);
        self.questions.entry(section_title.to_string())
            .or_insert_with(Vec::new)
            .extend(questions);
    }

    /// STUDY STEP: Ingest a section's content into LFI's knowledge.
    /// This is the "read the textbook" phase.
    pub fn study_section(
        &self,
        section_title: &str,
        knowledge: &mut KnowledgeEngine,
    ) -> Result<(), HdcError> {
        let section = self.sections.get(section_title)
            .ok_or_else(|| HdcError::InitializationFailed {
                reason: format!("Section '{}' not found", section_title),
            })?;

        debuglog!("TextbookLearner::study_section: '{}'", section_title);

        // Teach each concept with the full section content as definition.
        for concept in &section.concepts {
            knowledge.learn_with_definition(
                concept,
                &section.content,
                &[&section.topic],
                0.5, // Initial moderate mastery after reading
                true,
            )?;
        }

        Ok(())
    }

    /// TEST STEP: Ask LFI a question. LFI has access to knowledge (from reading),
    /// but NOT to the expected answer.
    ///
    /// `answer_fn` is the LFI's answer function: given a question, it produces
    /// an answer based on what it has learned. It does NOT receive the expected
    /// answer — that's how we prevent cheating via memorization.
    pub fn test_question<F>(
        &mut self,
        question: &TextbookQuestion,
        session: &mut StudySession,
        answer_fn: F,
    ) -> StudyAttempt
    where F: FnOnce(&str) -> (String, f64) {
        // LFI answers using ONLY the question + its learned knowledge.
        let (lfi_answer, confidence) = answer_fn(&question.question);

        // Grader verifies against the hidden expected answer.
        let mut acceptable = vec![question.expected_answer.clone()];
        acceptable.extend(question.acceptable_alternatives.iter().cloned());
        let acceptable_refs: Vec<&str> = acceptable.iter().map(|s| s.as_str()).collect();
        let verify = AnswerVerifier::verify_multi(&lfi_answer, &acceptable_refs);

        session.total += 1;
        if verify.is_correct {
            session.correct += 1;
        }

        let attempt = StudyAttempt {
            question: question.question.clone(),
            section_read: question.requires_section.clone(),
            lfi_answer: lfi_answer.clone(),
            was_correct: verify.is_correct,
            verification_mode: verify.matched_mode.clone(),
            confidence,
        };

        session.attempts.push(attempt.clone());
        attempt
    }

    /// RUN A FULL STUDY SESSION: study all sections, answer all questions.
    pub fn run_session<F>(
        &mut self,
        subject: &str,
        knowledge: &mut KnowledgeEngine,
        mut answer_fn: F,
    ) -> Result<StudySession, HdcError>
    where F: FnMut(&str) -> (String, f64) {
        let mut session = StudySession::new(subject);

        // Study all sections first (the "read the textbook" phase).
        let section_titles: Vec<String> = self.sections.keys().cloned().collect();
        for title in &section_titles {
            self.study_section(title, knowledge)?;
            session.concepts_read.push(title.clone());
        }

        // Test each question.
        let all_questions: Vec<(String, TextbookQuestion)> = self.questions.iter()
            .flat_map(|(section, qs)| {
                qs.iter().map(move |q| (section.clone(), q.clone()))
            })
            .collect();

        for (_section, question) in all_questions {
            let _ = self.test_question(&question, &mut session, &mut answer_fn);
        }

        self.sessions.push(session.clone());
        Ok(session)
    }

    /// Identify concepts LFI is struggling with (low correctness).
    pub fn weak_concepts(&self) -> Vec<String> {
        let mut counts: HashMap<String, (usize, usize)> = HashMap::new();
        for session in &self.sessions {
            for attempt in &session.attempts {
                let entry = counts.entry(attempt.section_read.clone()).or_insert((0, 0));
                entry.1 += 1;
                if attempt.was_correct { entry.0 += 1; }
            }
        }

        let mut weak: Vec<(String, f64)> = counts.into_iter()
            .map(|(k, (c, t))| (k, c as f64 / t.max(1) as f64))
            .filter(|(_, acc)| *acc < 0.7)
            .collect();
        weak.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        weak.into_iter().map(|(k, _)| k).collect()
    }

    /// Generate a study report.
    pub fn report(&self) -> String {
        let mut out = "=== Textbook Learning Report ===\n".to_string();
        out.push_str(&format!("Sections:  {}\n", self.sections.len()));
        let total_questions: usize = self.questions.values().map(|v| v.len()).sum();
        out.push_str(&format!("Questions: {}\n", total_questions));
        out.push_str(&format!("Sessions:  {}\n", self.sessions.len()));

        if let Some(latest) = self.sessions.last() {
            out.push_str(&format!("\nLatest session ({}):\n", latest.subject));
            out.push_str(&format!("  Accuracy: {:.1}%\n", latest.accuracy() * 100.0));
            out.push_str(&format!("  Attempts: {}/{}\n", latest.correct, latest.total));
        }

        let weak = self.weak_concepts();
        if !weak.is_empty() {
            out.push_str(&format!("\nWeak sections (need re-study):\n"));
            for section in weak.iter().take(5) {
                out.push_str(&format!("  - {}\n", section));
            }
        }

        out
    }
}

// ============================================================
// Sample Textbook Content
// ============================================================

pub struct SampleTextbooks;

impl SampleTextbooks {
    /// A mini-textbook on cryptographic hash functions.
    pub fn cryptography_basics() -> (Vec<TextbookSection>, Vec<(String, Vec<TextbookQuestion>)>) {
        let sections = vec![
            TextbookSection {
                title: "hash_functions".into(),
                topic: "crypto".into(),
                content: "A cryptographic hash function maps input data of arbitrary size to \
                         a fixed-size output called a hash or digest. Desired properties: \
                         (1) pre-image resistance (given hash, hard to find input), \
                         (2) second pre-image resistance (given input, hard to find another \
                         with same hash), (3) collision resistance (hard to find any two \
                         inputs with same hash). Common algorithms: SHA-256, SHA-3, BLAKE2. \
                         MD5 and SHA-1 are BROKEN and should not be used for security.".into(),
                concepts: vec!["hash_function".into(), "sha256".into(), "md5_broken".into()],
                difficulty: 0.3,
            },
            TextbookSection {
                title: "digital_signatures".into(),
                topic: "crypto".into(),
                content: "A digital signature proves authorship and integrity. Process: \
                         (1) sender computes hash of message, (2) encrypts hash with private key \
                         to create signature, (3) sends message + signature. Verifier: \
                         (1) decrypts signature with sender's public key to get original hash, \
                         (2) hashes the received message independently, \
                         (3) compares — if equal, signature is valid. Algorithms: RSA, ECDSA, Ed25519.".into(),
                concepts: vec!["digital_signature".into(), "rsa".into(), "ecdsa".into()],
                difficulty: 0.4,
            },
        ];

        let questions = vec![
            ("hash_functions".to_string(), vec![
                TextbookQuestion::new(
                    "What is pre-image resistance in a hash function?",
                    "hash_functions",
                    "hard to find input from hash",
                    0.3,
                ).with_alternatives(vec![
                    "difficult to reverse".into(),
                    "cannot easily find input given output".into(),
                    "one-way property".into(),
                ]),
                TextbookQuestion::new(
                    "Is MD5 safe to use for security?",
                    "hash_functions",
                    "no",
                    0.2,
                ).with_alternatives(vec!["not safe".into(), "broken".into(), "should not be used".into()]),
                TextbookQuestion::new(
                    "Name one modern cryptographic hash function that is still secure.",
                    "hash_functions",
                    "SHA-256",
                    0.25,
                ).with_alternatives(vec![
                    "sha256".into(), "sha-3".into(), "sha3".into(), "blake2".into(),
                ]),
            ]),
            ("digital_signatures".to_string(), vec![
                TextbookQuestion::new(
                    "Which key creates a digital signature — public or private?",
                    "digital_signatures",
                    "private",
                    0.3,
                ).with_alternatives(vec!["private key".into(), "the private key".into()]),
                TextbookQuestion::new(
                    "What two properties does a digital signature prove?",
                    "digital_signatures",
                    "authorship and integrity",
                    0.4,
                ).with_alternatives(vec![
                    "authenticity and integrity".into(),
                    "identity and tampering detection".into(),
                    "sender identity and message integrity".into(),
                ]),
            ]),
        ];

        (sections, questions)
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learner_creation() {
        let learner = TextbookLearner::new();
        assert!(learner.sections.is_empty());
        assert!(learner.questions.is_empty());
    }

    #[test]
    fn test_add_section_and_question() {
        let mut learner = TextbookLearner::new();

        learner.add_section(TextbookSection {
            title: "intro".into(),
            topic: "test".into(),
            content: "This is a test section about testing.".into(),
            concepts: vec!["testing".into()],
            difficulty: 0.1,
        });

        learner.add_questions("intro", vec![
            TextbookQuestion::new("What is this about?", "intro", "testing", 0.1),
        ]);

        assert_eq!(learner.sections.len(), 1);
        assert_eq!(learner.questions.get("intro").unwrap().len(), 1);
    }

    #[test]
    fn test_study_session_full_run() -> Result<(), HdcError> {
        let mut learner = TextbookLearner::new();
        let mut knowledge = KnowledgeEngine::new();

        let (sections, questions) = SampleTextbooks::cryptography_basics();
        for s in sections { learner.add_section(s); }
        for (title, qs) in questions {
            learner.add_questions(&title, qs);
        }

        // Simulate LFI answering: always returns the correct answer if present
        // in its knowledge (simulation — real LFI would query the knowledge engine).
        let session = learner.run_session("crypto_basics", &mut knowledge, |prompt| {
            // Trivial heuristic: if prompt mentions MD5, say "no"; otherwise be uncertain.
            if prompt.to_lowercase().contains("md5") {
                ("MD5 is not safe".into(), 0.9)
            } else if prompt.to_lowercase().contains("private or public") ||
                      prompt.to_lowercase().contains("public or private") {
                ("private".into(), 0.8)
            } else {
                ("I need to think more about this".into(), 0.3)
            }
        })?;

        // Should have run all questions.
        assert_eq!(session.total, 5);
        // Simulated answer function got MD5 + private = 2 correct.
        assert!(session.correct >= 1, "Should get at least 1 correct: {:?}",
            session.attempts.iter().map(|a| (&a.question, a.was_correct)).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_never_sees_expected_answer() {
        let mut learner = TextbookLearner::new();
        let mut knowledge = KnowledgeEngine::new();

        let q = TextbookQuestion::new(
            "What is the secret?",
            "intro",
            "SECRET_ANSWER_THAT_LFI_SHOULD_NEVER_SEE",
            0.3,
        );

        learner.add_section(TextbookSection {
            title: "intro".into(),
            topic: "test".into(),
            content: "public content only".into(),
            concepts: vec!["topic".into()],
            difficulty: 0.1,
        });

        let mut session = StudySession::new("test");

        // Track what the answer function receives.
        let mut received_prompt = String::new();
        let _ = learner.test_question(&q, &mut session, |prompt| {
            received_prompt = prompt.to_string();
            ("some answer".into(), 0.5)
        });

        // The answer function should NEVER see the expected answer.
        assert!(!received_prompt.contains("SECRET_ANSWER"),
            "Answer function must not see expected answer. Got: {}", received_prompt);
    }

    #[test]
    fn test_weak_concepts_identified() -> Result<(), HdcError> {
        let mut learner = TextbookLearner::new();
        let mut knowledge = KnowledgeEngine::new();

        let (sections, questions) = SampleTextbooks::cryptography_basics();
        for s in sections { learner.add_section(s); }
        for (title, qs) in questions {
            learner.add_questions(&title, qs);
        }

        // LFI always answers wrong.
        let _ = learner.run_session("failing", &mut knowledge, |_| {
            ("wrong answer".into(), 0.1)
        });

        let weak = learner.weak_concepts();
        // Both sections should be weak (both < 70% accuracy with all-wrong answers).
        assert!(!weak.is_empty(), "Should identify weak sections");
        Ok(())
    }

    #[test]
    fn test_report_generation() -> Result<(), HdcError> {
        let mut learner = TextbookLearner::new();
        let mut knowledge = KnowledgeEngine::new();
        let (sections, questions) = SampleTextbooks::cryptography_basics();
        for s in sections { learner.add_section(s); }
        for (title, qs) in questions {
            learner.add_questions(&title, qs);
        }

        let _ = learner.run_session("test", &mut knowledge, |_| ("answer".into(), 0.5))?;
        let report = learner.report();
        assert!(report.contains("Textbook Learning Report"));
        assert!(report.contains("Sections:"));
        Ok(())
    }

    #[test]
    fn test_sample_textbook_well_formed() {
        let (sections, questions) = SampleTextbooks::cryptography_basics();
        assert!(sections.len() >= 2);
        assert!(!questions.is_empty());
        for s in &sections {
            assert!(!s.content.is_empty());
            assert!(!s.concepts.is_empty());
        }
    }

    #[test]
    fn test_accuracy_calculation() {
        let mut session = StudySession::new("test");
        session.total = 10;
        session.correct = 7;
        assert!((session.accuracy() - 0.7).abs() < 0.01);

        let empty = StudySession::new("test");
        assert_eq!(empty.accuracy(), 0.0);
    }
}
