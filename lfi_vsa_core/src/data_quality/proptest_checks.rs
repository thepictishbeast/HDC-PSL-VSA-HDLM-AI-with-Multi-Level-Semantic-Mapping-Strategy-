// ============================================================
// Property-Based Tests for Data Quality Modules
// AVP-2 Tier 6: Meta-validation via proptest
//
// PURPOSE: Generate random inputs and verify invariants hold
// across thousands of cases. Complements unit tests by finding
// edge cases that hand-written tests miss.
// ============================================================

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    // ========== MinHash Properties ==========

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// MinHash similarity of identical shingle sets is always 1.0
        #[test]
        fn minhash_identical_similarity_is_one(
            text in "[a-zA-Z ]{20,200}"
        ) {
            use crate::data_quality::minhash::MinHashDedup;
            let dedup = MinHashDedup::new();
            let shingles = MinHashDedup::shingle(&text, 5);
            let sig1 = dedup.signature(&shingles);
            let sig2 = dedup.signature(&shingles);
            let sim = MinHashDedup::jaccard(&sig1, &sig2);
            prop_assert!((sim - 1.0).abs() < f64::EPSILON,
                "Identical shingles should have similarity 1.0, got {}", sim);
        }

        /// MinHash similarity is always between 0.0 and 1.0
        #[test]
        fn minhash_similarity_bounded(
            text1 in "[a-zA-Z ]{10,200}",
            text2 in "[a-zA-Z ]{10,200}",
        ) {
            use crate::data_quality::minhash::MinHashDedup;
            let dedup = MinHashDedup::new();
            let s1 = dedup.signature(&MinHashDedup::shingle(&text1, 5));
            let s2 = dedup.signature(&MinHashDedup::shingle(&text2, 5));
            let sim = MinHashDedup::jaccard(&s1, &s2);
            prop_assert!(sim >= 0.0 && sim <= 1.0,
                "Similarity must be in [0, 1], got {}", sim);
        }

        /// MinHash signature always has 128 hashes (configured default)
        #[test]
        fn minhash_signature_has_128_hashes(
            text in "[a-zA-Z ]{10,100}",
        ) {
            use crate::data_quality::minhash::MinHashDedup;
            let dedup = MinHashDedup::new();
            let sig = dedup.signature(&MinHashDedup::shingle(&text, 5));
            prop_assert_eq!(sig.hashes.len(), 128);
        }
    }

    // ========== Bloom Filter Properties ==========

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// A text added to the Bloom filter always tests as contaminated
        #[test]
        fn bloom_added_text_always_detected(
            text in "[a-zA-Z ]{15,200}"
        ) {
            use crate::data_quality::bloom::BloomDecontaminator;
            let mut bloom = BloomDecontaminator::new(10000);
            bloom.add_test_text(&text);
            let score = bloom.contamination_score(&text);
            prop_assert!(score > 0.8,
                "Added text should have high contamination score, got {}", score);
        }

        /// Contamination score is always between 0.0 and 1.0
        #[test]
        fn bloom_score_bounded(
            text in "[a-zA-Z ]{5,200}"
        ) {
            use crate::data_quality::bloom::BloomDecontaminator;
            let bloom = BloomDecontaminator::new(10000);
            let score = bloom.contamination_score(&text);
            prop_assert!(score >= 0.0 && score <= 1.0,
                "Score must be in [0, 1], got {}", score);
        }
    }

    // ========== Temporal Decay Properties ==========

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2000))]

        /// Decay-adjusted quality never exceeds base quality
        #[test]
        fn temporal_decayed_never_exceeds_base(
            base_quality in 0.0f64..1.0,
            age_days in 0.0f64..10000.0,
        ) {
            use crate::data_quality::temporal::TemporalDecay;
            let td = TemporalDecay::new();
            let decayed = td.adjusted_quality(base_quality, "general", age_days);
            prop_assert!(decayed <= base_quality + f64::EPSILON,
                "Decayed {} should not exceed base {}", decayed, base_quality);
        }

        /// Decay-adjusted quality is always non-negative
        #[test]
        fn temporal_decayed_non_negative(
            base_quality in 0.0f64..1.0,
            age_days in 0.0f64..100000.0,
        ) {
            use crate::data_quality::temporal::TemporalDecay;
            let td = TemporalDecay::new();
            let decayed = td.adjusted_quality(base_quality, "cybersecurity", age_days);
            prop_assert!(decayed >= 0.0,
                "Decayed quality must be non-negative, got {}", decayed);
        }

        /// Older facts have lower or equal quality than newer facts (monotonic decay)
        #[test]
        fn temporal_monotonic_decay(
            base_quality in 0.01f64..1.0,
            age1 in 0.0f64..5000.0,
            delta in 0.0f64..5000.0,
        ) {
            use crate::data_quality::temporal::TemporalDecay;
            let td = TemporalDecay::new();
            let age2 = age1 + delta;
            let q1 = td.adjusted_quality(base_quality, "technology", age1);
            let q2 = td.adjusted_quality(base_quality, "technology", age2);
            prop_assert!(q2 <= q1 + f64::EPSILON,
                "Older fact ({} days, q={}) should not exceed newer ({} days, q={})",
                age2, q2, age1, q1);
        }
    }

    // ========== Quality Classifier Properties ==========

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Quality score is always between 0.0 and 1.0
        #[test]
        fn classifier_score_bounded(
            text in ".{1,500}"
        ) {
            use crate::data_quality::classifier::QualityClassifier;
            let qc = QualityClassifier::new();
            let score = qc.score(&text);
            prop_assert!(score >= 0.0 && score <= 1.0,
                "Score must be in [0, 1], got {} for text len={}", score, text.len());
        }

        /// Empty string always scores 0.0
        #[test]
        fn classifier_whitespace_scores_zero(
            n in 0usize..50
        ) {
            use crate::data_quality::classifier::QualityClassifier;
            let qc = QualityClassifier::new();
            let spaces = " ".repeat(n);
            let score = qc.score(&spaces);
            prop_assert!((score - 0.0).abs() < f64::EPSILON,
                "Whitespace-only should score 0.0, got {}", score);
        }
    }

    // ========== Pipeline Properties ==========

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// Pipeline verdict quality_score matches classifier score
        #[test]
        fn pipeline_score_matches_classifier(
            text in "[a-zA-Z .,!?]{20,300}"
        ) {
            use crate::data_quality::pipeline::QualityPipeline;
            use crate::data_quality::classifier::QualityClassifier;
            let pipeline = QualityPipeline::new();
            let classifier = QualityClassifier::new();
            let verdict = pipeline.evaluate(&text, "general", None);
            let direct_score = classifier.score(&text);
            prop_assert!((verdict.quality_score - direct_score).abs() < f64::EPSILON,
                "Pipeline score {} should match classifier score {}",
                verdict.quality_score, direct_score);
        }

        /// Pipeline never panics on arbitrary input
        #[test]
        fn pipeline_no_panic_on_arbitrary_input(
            text in ".*",
            domain in "[a-z]{3,15}",
            age in proptest::option::of(0.0f64..10000.0),
        ) {
            use crate::data_quality::pipeline::QualityPipeline;
            let pipeline = QualityPipeline::new();
            let _verdict = pipeline.evaluate(&text, &domain, age);
            // If we get here without panicking, the test passes
        }
    }
}
