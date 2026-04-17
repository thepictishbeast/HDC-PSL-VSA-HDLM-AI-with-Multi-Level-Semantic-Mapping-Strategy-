// ============================================================
// Emotion Detector — Sentiment & Emotional State Analysis
//
// Detects user emotional state from text to adapt response tone.
// Frustrated users get empathetic, solution-focused responses.
// Excited users get enthusiastic engagement.
// Confused users get clearer, step-by-step explanations.
//
// SUPERSOCIETY: Emotional intelligence is the difference between
// a tool and a companion. Read the room. Adapt the tone.
// ============================================================

/// Detected emotional states.
#[derive(Debug, Clone, PartialEq)]
pub enum Emotion {
    Neutral,
    Frustrated,
    Excited,
    Confused,
    Curious,
    Appreciative,
    Urgent,
    Playful,
    Sad,
}

impl Emotion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Frustrated => "frustrated",
            Self::Excited => "excited",
            Self::Confused => "confused",
            Self::Curious => "curious",
            Self::Appreciative => "appreciative",
            Self::Urgent => "urgent",
            Self::Playful => "playful",
            Self::Sad => "sad",
        }
    }

    /// Get tone guidance for the AI based on detected emotion.
    pub fn tone_guidance(&self) -> &'static str {
        match self {
            Self::Neutral => "Respond naturally and directly.",
            Self::Frustrated => "Be empathetic and solution-focused. Acknowledge the frustration. Skip pleasantries. Get to the answer fast.",
            Self::Excited => "Match their energy! Be enthusiastic. Encourage. Celebrate their win or discovery.",
            Self::Confused => "Use simple language. Break it into steps. Give concrete examples. Ask clarifying questions if needed.",
            Self::Curious => "Feed the curiosity! Go deeper. Share interesting details. Suggest related topics to explore.",
            Self::Appreciative => "Accept the thanks gracefully. Offer to help with anything else. Be warm.",
            Self::Urgent => "Be concise and direct. Skip the preamble. Give the critical information first.",
            Self::Playful => "Have fun! Use humor. Be witty. But still be helpful.",
            Self::Sad => "Be gentle and supportive. Don't try to fix the emotion. Listen and respond with care.",
        }
    }
}

/// Analysis result with confidence.
#[derive(Debug, Clone)]
pub struct EmotionAnalysis {
    pub primary: Emotion,
    pub confidence: f64,
    pub secondary: Option<Emotion>,
    pub signals: Vec<String>,
}

/// Detect emotion from user input text.
/// Uses pattern matching and sentiment heuristics — no ML model needed.
pub fn detect_emotion(input: &str) -> EmotionAnalysis {
    let lower = input.to_lowercase();
    let len = input.len();

    let mut scores: Vec<(Emotion, f64, Vec<String>)> = Vec::new();

    // Frustration signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let frustration_words = ["broken", "doesn't work", "not working", "wrong", "terrible",
            "awful", "useless", "stupid", "hate", "can't", "won't", "failed", "failing",
            "error", "bug", "crash", "fix this", "help me", "frustrated", "annoying",
            "ridiculous", "impossible", "give up", "waste of time", "again"];
        for word in &frustration_words {
            if lower.contains(word) { score += 0.15; signals.push(format!("'{}'", word)); }
        }
        // Exclamation marks amplify frustration
        let excl = input.matches('!').count();
        if excl >= 2 { score += 0.1; signals.push("multiple !".into()); }
        // ALL CAPS
        let caps_ratio = input.chars().filter(|c| c.is_uppercase()).count() as f64 / len.max(1) as f64;
        if caps_ratio > 0.5 && len > 10 { score += 0.2; signals.push("ALL CAPS".into()); }
        scores.push((Emotion::Frustrated, score.min(1.0), signals));
    }

    // Excitement signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["awesome", "amazing", "incredible", "love", "great", "perfect",
            "brilliant", "fantastic", "excellent", "wow", "cool", "nice", "beautiful",
            "wonderful", "exciting", "yes!", "yay", "woohoo", "finally", "it works"];
        for w in &words {
            if lower.contains(w) { score += 0.15; signals.push(format!("'{}'", w)); }
        }
        let excl = input.matches('!').count();
        if excl >= 1 && !lower.contains("not") { score += 0.1; signals.push("!".into()); }
        scores.push((Emotion::Excited, score.min(1.0), signals));
    }

    // Confusion signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["confused", "don't understand", "what do you mean", "unclear",
            "lost", "huh", "???", "makes no sense", "what?", "how?", "why?",
            "explain", "clarify", "i'm not sure", "what is", "can you explain"];
        for w in &words {
            if lower.contains(w) { score += 0.15; signals.push(format!("'{}'", w)); }
        }
        let q_marks = input.matches('?').count();
        if q_marks >= 2 { score += 0.1; signals.push("multiple ?".into()); }
        scores.push((Emotion::Confused, score.min(1.0), signals));
    }

    // Curiosity signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["interesting", "curious", "wondering", "tell me more",
            "how does", "why does", "what if", "could you explain", "i'd like to know",
            "fascinating", "learn about", "teach me", "show me"];
        for w in &words {
            if lower.contains(w) { score += 0.15; signals.push(format!("'{}'", w)); }
        }
        // Single question mark with moderate length = curious
        if input.contains('?') && len > 30 && len < 200 { score += 0.1; }
        scores.push((Emotion::Curious, score.min(1.0), signals));
    }

    // Appreciation signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["thanks", "thank you", "appreciate", "grateful", "helpful",
            "you're the best", "that helped", "perfect answer", "exactly what i needed"];
        for w in &words {
            if lower.contains(w) { score += 0.2; signals.push(format!("'{}'", w)); }
        }
        scores.push((Emotion::Appreciative, score.min(1.0), signals));
    }

    // Urgency signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["urgent", "asap", "immediately", "right now", "emergency",
            "deadline", "quickly", "hurry", "critical", "need this now", "production is down"];
        for w in &words {
            if lower.contains(w) { score += 0.2; signals.push(format!("'{}'", w)); }
        }
        scores.push((Emotion::Urgent, score.min(1.0), signals));
    }

    // Playful signals
    {
        let mut score = 0.0f64;
        let mut signals = Vec::new();
        let words = ["haha", "lol", "lmao", "joke", "funny", "😂", "😄",
            "just kidding", "for fun", "play", "game"];
        for w in &words {
            if lower.contains(w) { score += 0.15; signals.push(format!("'{}'", w)); }
        }
        scores.push((Emotion::Playful, score.min(1.0), signals));
    }

    // Sort by score descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (primary, confidence, signals) = if scores[0].1 > 0.1 {
        (scores[0].0.clone(), scores[0].1, scores[0].2.clone())
    } else {
        (Emotion::Neutral, 0.5, vec!["no strong signals".into()])
    };

    let secondary = if scores.len() > 1 && scores[1].1 > 0.1 {
        Some(scores[1].0.clone())
    } else {
        None
    };

    EmotionAnalysis {
        primary,
        confidence,
        secondary,
        signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustrated() {
        let r = detect_emotion("This is broken! Nothing works and I hate it!");
        assert_eq!(r.primary, Emotion::Frustrated);
        assert!(r.confidence > 0.3);
    }

    #[test]
    fn test_excited() {
        let r = detect_emotion("Wow that's awesome! It finally works perfectly!");
        assert_eq!(r.primary, Emotion::Excited);
    }

    #[test]
    fn test_confused() {
        let r = detect_emotion("I don't understand what you mean? Can you explain???");
        assert_eq!(r.primary, Emotion::Confused);
    }

    #[test]
    fn test_curious() {
        let r = detect_emotion("I'm curious about how quantum computers work. Tell me more about the quantum entanglement process.");
        assert_eq!(r.primary, Emotion::Curious);
    }

    #[test]
    fn test_appreciative() {
        let r = detect_emotion("Thank you so much! That was incredibly helpful!");
        assert_eq!(r.primary, Emotion::Appreciative);
    }

    #[test]
    fn test_neutral() {
        let r = detect_emotion("What time is it?");
        assert_eq!(r.primary, Emotion::Neutral);
    }

    #[test]
    fn test_urgent() {
        let r = detect_emotion("URGENT: production is down, need fix immediately!");
        assert!(r.primary == Emotion::Urgent || r.primary == Emotion::Frustrated);
    }

    #[test]
    fn test_tone_guidance() {
        assert!(Emotion::Frustrated.tone_guidance().contains("empathetic"));
        assert!(Emotion::Curious.tone_guidance().contains("deeper"));
    }
}
