// ============================================================
// Conversation Summarizer — Compress long conversations
//
// After 20+ messages, summarizes the conversation into a compact
// representation. Used for:
// 1. Context window management (keep recent + summary)
// 2. Cross-session continuity (load summary on reconnect)
// 3. Training data extraction (conversation → instruction pairs)
//
// SUPERSOCIETY: The difference between a goldfish and a companion
// is memory. Summaries are compressed memories.
// ============================================================

/// A compressed conversation summary.
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub message_count: usize,
    pub summary: String,
    pub topics: Vec<String>,
    pub user_intent: String,
    pub key_facts_learned: Vec<String>,
    pub unresolved_questions: Vec<String>,
}

/// Summarize a conversation from a list of (role, content) messages.
/// BUG ASSUMPTION: This is heuristic extraction, not LLM-based.
/// For high-quality summaries, pipe through Ollama.
pub fn summarize_conversation(
    conversation_id: &str,
    messages: &[(String, String)],
) -> ConversationSummary {
    let mut topics = Vec::new();
    let mut key_facts = Vec::new();
    let mut unresolved = Vec::new();
    let mut user_messages = Vec::new();
    let mut ai_messages = Vec::new();

    for (role, content) in messages {
        if role == "user" {
            user_messages.push(content.as_str());
            // Extract topics from questions
            let lower = content.to_lowercase();
            for keyword in extract_topic_keywords(content) {
                if !topics.contains(&keyword) && topics.len() < 10 {
                    topics.push(keyword);
                }
            }
            // Detect unresolved questions (last few user messages with ?)
            if content.contains('?') && messages.len() > 0 {
                let trimmed = content.lines().next().unwrap_or("").trim();
                if trimmed.len() > 10 && trimmed.len() < 200 {
                    unresolved.push(trimmed.to_string());
                }
            }
        } else {
            ai_messages.push(content.as_str());
            // Extract facts the AI learned about the user
            let lower = content.to_lowercase();
            if lower.contains("your name") || lower.contains("you mentioned") ||
               lower.contains("you said") || lower.contains("based on what you") {
                let first_line = content.lines().next().unwrap_or("");
                if first_line.len() > 10 && first_line.len() < 200 {
                    key_facts.push(first_line.to_string());
                }
            }
        }
    }

    // Keep only last 3 unresolved questions
    if unresolved.len() > 3 {
        unresolved = unresolved[unresolved.len()-3..].to_vec();
    }

    // Build summary from first + last user messages + topic keywords
    let user_intent = if let Some(first) = user_messages.first() {
        let truncated = if first.len() > 100 { &first[..100] } else { first };
        format!("Started with: \"{}\"", truncated)
    } else {
        "No user messages".to_string()
    };

    let summary = build_summary(&user_messages, &ai_messages, &topics);

    ConversationSummary {
        conversation_id: conversation_id.to_string(),
        message_count: messages.len(),
        summary,
        topics,
        user_intent,
        key_facts_learned: key_facts,
        unresolved_questions: unresolved,
    }
}

/// Extract topic keywords from user input.
fn extract_topic_keywords(text: &str) -> Vec<String> {
    let stopwords = ["the", "and", "for", "are", "but", "not", "you", "all",
        "can", "had", "was", "one", "has", "its", "how", "who", "what",
        "this", "that", "with", "from", "they", "been", "have", "will",
        "each", "make", "like", "does", "into", "than", "them", "some",
        "about", "would", "could", "should", "there", "their", "other",
        "just", "also", "more", "very", "much", "tell", "know", "want",
        "need", "help", "please", "think", "going", "really"];
    let stop_set: std::collections::HashSet<&str> = stopwords.iter().copied().collect();

    text.split_whitespace()
        .filter(|w| w.len() >= 4 && !stop_set.contains(w.to_lowercase().as_str()))
        .take(5)
        .map(|w| w.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>())
        .filter(|w| w.len() >= 4)
        .collect()
}

/// Build a concise summary from conversation content.
fn build_summary(user_msgs: &[&str], ai_msgs: &[&str], topics: &[String]) -> String {
    let total = user_msgs.len() + ai_msgs.len();

    let topic_str = if topics.is_empty() {
        "general conversation".to_string()
    } else {
        topics.join(", ")
    };

    let first_q = user_msgs.first().map(|m| {
        let s = if m.len() > 80 { &m[..80] } else { m };
        s.to_string()
    }).unwrap_or_default();

    let last_q = if user_msgs.len() > 1 {
        user_msgs.last().map(|m| {
            let s = if m.len() > 80 { &m[..80] } else { m };
            format!(" Most recently discussed: \"{}\"", s)
        }).unwrap_or_default()
    } else {
        String::new()
    };

    format!(
        "{} messages about {}. User started with: \"{}\"{}",
        total, topic_str, first_q, last_q
    )
}

/// Generate a compact context block from a summary for injection into prompts.
/// This replaces loading 20+ full messages — keeps the AI informed with ~200 tokens.
pub fn summary_to_context(summary: &ConversationSummary) -> String {
    let mut ctx = format!("Previous conversation summary ({} messages):\n", summary.message_count);
    ctx.push_str(&format!("Topics: {}\n", summary.topics.join(", ")));
    ctx.push_str(&format!("{}\n", summary.user_intent));

    if !summary.key_facts_learned.is_empty() {
        ctx.push_str("Key facts from conversation:\n");
        for fact in summary.key_facts_learned.iter().take(5) {
            ctx.push_str(&format!("- {}\n", fact));
        }
    }

    if !summary.unresolved_questions.is_empty() {
        ctx.push_str("Unresolved questions:\n");
        for q in &summary.unresolved_questions {
            ctx.push_str(&format!("- {}\n", q));
        }
    }

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_basic() {
        let messages = vec![
            ("user".to_string(), "How does TCP/IP work?".to_string()),
            ("assistant".to_string(), "TCP/IP is a layered protocol suite...".to_string()),
            ("user".to_string(), "What about UDP?".to_string()),
            ("assistant".to_string(), "UDP is a simpler, connectionless protocol...".to_string()),
        ];
        let summary = summarize_conversation("test-123", &messages);
        assert_eq!(summary.message_count, 4);
        assert!(!summary.topics.is_empty());
        assert!(summary.summary.contains("4 messages"));
    }

    #[test]
    fn test_summarize_empty() {
        let summary = summarize_conversation("empty", &[]);
        assert_eq!(summary.message_count, 0);
        assert_eq!(summary.user_intent, "No user messages");
    }

    #[test]
    fn test_topic_extraction() {
        let topics = extract_topic_keywords("How does quantum computing affect cryptography?");
        assert!(topics.iter().any(|t| t.contains("quantum") || t.contains("computing") || t.contains("cryptography")));
    }

    #[test]
    fn test_summary_to_context() {
        let summary = ConversationSummary {
            conversation_id: "test".to_string(),
            message_count: 10,
            summary: "10 messages about networking".to_string(),
            topics: vec!["networking".to_string(), "security".to_string()],
            user_intent: "Started with: \"How do firewalls work?\"".to_string(),
            key_facts_learned: vec!["User is a network engineer".to_string()],
            unresolved_questions: vec!["What about IDS/IPS?".to_string()],
        };
        let ctx = summary_to_context(&summary);
        assert!(ctx.contains("10 messages"));
        assert!(ctx.contains("networking"));
        assert!(ctx.contains("IDS/IPS"));
    }
}
