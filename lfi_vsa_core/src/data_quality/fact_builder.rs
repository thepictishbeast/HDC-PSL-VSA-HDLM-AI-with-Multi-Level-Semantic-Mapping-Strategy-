// ============================================================
// FactBuilder — Ergonomic fact construction with validation
// 500-task list item 417: Builder pattern for fact insertion
//
// PURPOSE: Provide a safe, readable API for constructing facts
// that enforces validation at build time rather than at DB insert.
// Usage: FactBuilder::new("key").value("val").domain("science").build()
// ============================================================

use super::domain::Domain;
use super::source::Source;

/// A validated fact ready for insertion into brain.db.
#[derive(Debug, Clone)]
pub struct Fact {
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub source: Source,
    pub domain: Domain,
    pub quality_score: f64,
}

/// Builder for constructing validated Facts.
///
/// ```ignore
/// let fact = FactBuilder::new("cve_2024_1234")
///     .value("Remote code execution in Apache Struts...")
///     .domain("cybersecurity")
///     .source("cve_database")
///     .confidence(0.95)
///     .build()?;
/// ```
pub struct FactBuilder {
    key: String,
    value: Option<String>,
    confidence: f64,
    source: Option<Source>,
    domain: Option<Domain>,
    quality_score: Option<f64>,
}

impl FactBuilder {
    /// Start building a fact with a required key.
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            value: None,
            confidence: 0.7,
            source: None,
            domain: None,
            quality_score: None,
        }
    }

    /// Set the fact value (required).
    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    /// Set the domain.
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = Some(Domain::new(domain));
        self
    }

    /// Set the source.
    pub fn source(mut self, source: &str) -> Self {
        self.source = Some(Source::new(source));
        self
    }

    /// Set confidence (0.0 - 1.0).
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set quality score (0.0 - 1.0). If not set, inferred from source tier.
    pub fn quality(mut self, score: f64) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }

    /// Build the fact, validating all fields.
    pub fn build(self) -> Result<Fact, String> {
        let value = self.value.ok_or("Fact value is required")?;

        if self.key.is_empty() {
            return Err("Fact key cannot be empty".into());
        }
        if self.key.len() > 512 {
            return Err("Fact key too long (max 512 bytes)".into());
        }
        if value.is_empty() {
            return Err("Fact value cannot be empty".into());
        }
        if value.len() > 100_000 {
            return Err("Fact value too long (max 100KB)".into());
        }

        let source = self.source.unwrap_or_else(|| Source::new("unknown"));
        let domain = self.domain.unwrap_or_else(|| Domain::new("general"));

        // Auto-infer quality from source tier if not explicitly set
        let quality_score = self.quality_score
            .unwrap_or_else(|| source.quality_floor());

        Ok(Fact {
            key: self.key,
            value,
            confidence: self.confidence,
            source,
            domain,
            quality_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_build() {
        let fact = FactBuilder::new("test_key")
            .value("test value")
            .domain("science")
            .source("handcrafted")
            .confidence(0.9)
            .build()
            .unwrap();

        assert_eq!(fact.key, "test_key");
        assert_eq!(fact.value, "test value");
        assert_eq!(fact.domain.as_str(), "science");
        assert_eq!(fact.source.as_str(), "handcrafted");
        assert_eq!(fact.confidence, 0.9);
    }

    #[test]
    fn test_defaults() {
        let fact = FactBuilder::new("k")
            .value("v")
            .build()
            .unwrap();

        assert_eq!(fact.domain.as_str(), "general");
        assert_eq!(fact.source.as_str(), "unknown");
        assert_eq!(fact.confidence, 0.7);
    }

    #[test]
    fn test_quality_inferred_from_source() {
        let fact = FactBuilder::new("k")
            .value("v")
            .source("handcrafted_expert")
            .build()
            .unwrap();

        assert!(fact.quality_score >= 0.8,
            "Authoritative source should have high quality floor: {}", fact.quality_score);
    }

    #[test]
    fn test_explicit_quality_overrides() {
        let fact = FactBuilder::new("k")
            .value("v")
            .source("handcrafted")
            .quality(0.5)
            .build()
            .unwrap();

        assert_eq!(fact.quality_score, 0.5);
    }

    #[test]
    fn test_missing_value_errors() {
        let result = FactBuilder::new("k").build();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("value is required"));
    }

    #[test]
    fn test_empty_key_errors() {
        let result = FactBuilder::new("").value("v").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_value_errors() {
        let result = FactBuilder::new("k").value("").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_key_too_long_errors() {
        let long_key = "x".repeat(513);
        let result = FactBuilder::new(&long_key).value("v").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_confidence_clamped() {
        let fact = FactBuilder::new("k")
            .value("v")
            .confidence(5.0)
            .build()
            .unwrap();
        assert_eq!(fact.confidence, 1.0);

        let fact2 = FactBuilder::new("k")
            .value("v")
            .confidence(-1.0)
            .build()
            .unwrap();
        assert_eq!(fact2.confidence, 0.0);
    }

    #[test]
    fn test_quality_clamped() {
        let fact = FactBuilder::new("k")
            .value("v")
            .quality(2.0)
            .build()
            .unwrap();
        assert_eq!(fact.quality_score, 1.0);
    }

    #[test]
    fn test_chaining() {
        // Verify the builder pattern chains work
        let result = FactBuilder::new("k")
            .value("some long value")
            .domain("cybersecurity")
            .source("oasst2_en_quality")
            .confidence(0.85)
            .quality(0.9)
            .build();
        assert!(result.is_ok());
    }
}
