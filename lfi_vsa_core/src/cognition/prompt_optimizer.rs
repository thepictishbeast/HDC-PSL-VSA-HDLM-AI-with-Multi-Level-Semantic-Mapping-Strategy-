// ============================================================
// Prompt Optimizer — Self-improving prompt templates
//
// Evaluates prompt template effectiveness based on response quality
// and user feedback. Keeps the template that produces best results.
//
// Approach: A/B test prompt variants, track which gets higher
// user satisfaction (fewer corrections, more positive feedback).
// ============================================================

use std::collections::HashMap;

/// A prompt template variant with performance tracking.
#[derive(Debug, Clone)]
pub struct PromptVariant {
    pub id: String,
    pub template: String,
    pub uses: usize,
    pub positive_feedback: usize,
    pub negative_feedback: usize,
    pub avg_confidence: f64,
    pub total_confidence: f64,
}

impl PromptVariant {
    pub fn new(id: &str, template: &str) -> Self {
        Self {
            id: id.to_string(),
            template: template.to_string(),
            uses: 0,
            positive_feedback: 0,
            negative_feedback: 0,
            avg_confidence: 0.0,
            total_confidence: 0.0,
        }
    }

    /// Effectiveness score: positive ratio weighted by sample size.
    pub fn effectiveness(&self) -> f64 {
        if self.uses < 5 {
            return 0.5; // Not enough data — neutral
        }
        let total_feedback = self.positive_feedback + self.negative_feedback;
        if total_feedback == 0 {
            return self.avg_confidence; // No explicit feedback — use confidence
        }
        let positive_ratio = self.positive_feedback as f64 / total_feedback as f64;
        // Blend with confidence: 70% user feedback, 30% model confidence
        positive_ratio * 0.7 + self.avg_confidence * 0.3
    }
}

/// Manages prompt template variants and selects the best.
pub struct PromptOptimizer {
    /// Variants keyed by category (e.g., "system_prompt", "search_prompt").
    variants: HashMap<String, Vec<PromptVariant>>,
    /// Which variant is currently active per category.
    active: HashMap<String, usize>,
}

impl PromptOptimizer {
    pub fn new() -> Self {
        let mut optimizer = Self {
            variants: HashMap::new(),
            active: HashMap::new(),
        };

        // Seed with default system prompt variants
        optimizer.add_variant("system_prompt", PromptVariant::new(
            "default",
            "You are PlausiDen AI, a sovereign intelligence. Answer directly and accurately."
        ));
        optimizer.add_variant("system_prompt", PromptVariant::new(
            "detailed",
            "You are PlausiDen AI. Provide thorough, well-structured answers with examples. Be specific."
        ));
        optimizer.add_variant("system_prompt", PromptVariant::new(
            "concise",
            "You are PlausiDen AI. Be concise and direct. Answer in the fewest words that fully address the question."
        ));

        optimizer
    }

    /// Add a variant to a category.
    pub fn add_variant(&mut self, category: &str, variant: PromptVariant) {
        self.variants.entry(category.to_string()).or_default().push(variant);
        self.active.entry(category.to_string()).or_insert(0);
    }

    /// Get the currently active prompt for a category.
    pub fn active_prompt(&self, category: &str) -> Option<&str> {
        let variants = self.variants.get(category)?;
        let idx = self.active.get(category).copied().unwrap_or(0);
        variants.get(idx).map(|v| v.template.as_str())
    }

    /// Record that the active variant was used with a given confidence.
    pub fn record_use(&mut self, category: &str, confidence: f64) {
        let idx = self.active.get(category).copied().unwrap_or(0);
        if let Some(variants) = self.variants.get_mut(category) {
            if let Some(v) = variants.get_mut(idx) {
                v.uses += 1;
                v.total_confidence += confidence;
                v.avg_confidence = v.total_confidence / v.uses as f64;
            }
        }
    }

    /// Record positive feedback for the active variant.
    pub fn record_positive(&mut self, category: &str) {
        let idx = self.active.get(category).copied().unwrap_or(0);
        if let Some(variants) = self.variants.get_mut(category) {
            if let Some(v) = variants.get_mut(idx) {
                v.positive_feedback += 1;
            }
        }
    }

    /// Record negative feedback for the active variant.
    pub fn record_negative(&mut self, category: &str) {
        let idx = self.active.get(category).copied().unwrap_or(0);
        if let Some(variants) = self.variants.get_mut(category) {
            if let Some(v) = variants.get_mut(idx) {
                v.negative_feedback += 1;
            }
        }
    }

    /// Select the best variant based on effectiveness scores.
    /// Call periodically (e.g., every 50 uses) to update the active variant.
    pub fn optimize(&mut self, category: &str) {
        if let Some(variants) = self.variants.get(category) {
            if let Some((best_idx, _)) = variants.iter().enumerate()
                .max_by(|a, b| a.1.effectiveness().partial_cmp(&b.1.effectiveness())
                    .unwrap_or(std::cmp::Ordering::Equal))
            {
                self.active.insert(category.to_string(), best_idx);
            }
        }
    }

    /// Get performance report for all variants in a category.
    pub fn report(&self, category: &str) -> Vec<(String, f64, usize)> {
        self.variants.get(category).map(|vs| {
            vs.iter().map(|v| (v.id.clone(), v.effectiveness(), v.uses)).collect()
        }).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_variants() {
        let opt = PromptOptimizer::new();
        assert!(opt.active_prompt("system_prompt").is_some());
    }

    #[test]
    fn test_record_and_optimize() {
        let mut opt = PromptOptimizer::new();

        // Use default many times with good feedback
        for _ in 0..10 {
            opt.record_use("system_prompt", 0.8);
            opt.record_positive("system_prompt");
        }

        // Switch to "detailed" and get bad feedback
        opt.active.insert("system_prompt".to_string(), 1);
        for _ in 0..10 {
            opt.record_use("system_prompt", 0.5);
            opt.record_negative("system_prompt");
        }

        // Optimize should pick "default" (index 0)
        opt.optimize("system_prompt");
        assert_eq!(*opt.active.get("system_prompt").unwrap(), 0);
    }

    #[test]
    fn test_effectiveness_with_few_samples() {
        let v = PromptVariant::new("test", "template");
        assert_eq!(v.effectiveness(), 0.5); // Not enough data
    }

    #[test]
    fn test_report() {
        let opt = PromptOptimizer::new();
        let report = opt.report("system_prompt");
        assert_eq!(report.len(), 3);
    }
}
