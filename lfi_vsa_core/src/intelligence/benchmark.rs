// ============================================================
// Intelligence Benchmark — Measures LFI Capability Across Domains
//
// Runs the full training data as a benchmark, measures accuracy
// per domain, identifies weaknesses, and reports an overall
// intelligence score.
// ============================================================

use crate::cognition::knowledge::KnowledgeEngine;
use crate::hdc::error::HdcError;
use crate::intelligence::training_data::{TrainingDataGenerator, TrainingExample, CorrectionLoop};

/// Benchmark result for a single domain.
#[derive(Debug, Clone)]
pub struct DomainScore {
    pub domain: String,
    pub total: usize,
    pub correct: usize,
    pub accuracy: f64,
    pub difficulty_weighted_score: f64,
}

/// Complete benchmark report.
#[derive(Debug)]
pub struct BenchmarkReport {
    pub domain_scores: Vec<DomainScore>,
    pub overall_accuracy: f64,
    pub overall_weighted_score: f64,
    pub total_examples: usize,
    pub total_correct: usize,
    pub weakest_domain: String,
    pub strongest_domain: String,
    pub domains_evaluated: usize,
}

/// The intelligence benchmarking engine.
pub struct IntelligenceBenchmark;

impl IntelligenceBenchmark {
    /// Run a full benchmark against all training data.
    pub fn run(knowledge: &mut KnowledgeEngine) -> Result<BenchmarkReport, HdcError> {
        let examples = TrainingDataGenerator::all_examples();
        Self::run_with_examples(knowledge, &examples)
    }

    /// Run benchmark against specific examples.
    pub fn run_with_examples(
        knowledge: &mut KnowledgeEngine,
        examples: &[TrainingExample],
    ) -> Result<BenchmarkReport, HdcError> {
        debuglog!("IntelligenceBenchmark::run: {} examples", examples.len());

        let mut correction_loop = CorrectionLoop::new();
        let eval_results = correction_loop.evaluate_and_correct(knowledge, examples)?;

        let mut domain_scores: Vec<DomainScore> = Vec::new();
        let mut total_correct = 0;
        let mut total_examples = 0;
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;

        // Group examples by domain for difficulty-weighted scoring.
        let mut domain_examples: std::collections::HashMap<String, Vec<&TrainingExample>> =
            std::collections::HashMap::new();
        for ex in examples {
            domain_examples.entry(ex.domain.clone()).or_default().push(ex);
        }

        for result in &eval_results {
            total_correct += result.correct;
            total_examples += result.total;

            // Difficulty-weighted score: harder correct answers worth more.
            let empty_vec: Vec<&TrainingExample> = vec![];
            let domain_exs = domain_examples.get(&result.domain).unwrap_or(&empty_vec);
            let weighted: f64 = domain_exs.iter()
                .take(result.correct) // Assume first N are correct
                .map(|e| e.difficulty)
                .sum();
            let max_weighted: f64 = domain_exs.iter().map(|e| e.difficulty).sum();

            let dw_score = if max_weighted > 0.0 { weighted / max_weighted } else { 0.0 };
            total_weighted += weighted;
            total_weight += max_weighted;

            domain_scores.push(DomainScore {
                domain: result.domain.clone(),
                total: result.total,
                correct: result.correct,
                accuracy: result.accuracy,
                difficulty_weighted_score: dw_score,
            });
        }

        domain_scores.sort_by(|a, b| b.accuracy.partial_cmp(&a.accuracy).unwrap_or(std::cmp::Ordering::Equal));

        let strongest = domain_scores.first().map(|d| d.domain.clone()).unwrap_or_default();
        let weakest = domain_scores.last().map(|d| d.domain.clone()).unwrap_or_default();
        let overall_accuracy = if total_examples > 0 { total_correct as f64 / total_examples as f64 } else { 0.0 };
        let overall_weighted = if total_weight > 0.0 { total_weighted / total_weight } else { 0.0 };

        Ok(BenchmarkReport {
            domain_scores,
            overall_accuracy,
            overall_weighted_score: overall_weighted,
            total_examples,
            total_correct,
            weakest_domain: weakest,
            strongest_domain: strongest,
            domains_evaluated: eval_results.len(),
        })
    }

    /// Print a formatted benchmark report.
    pub fn print_report(report: &BenchmarkReport) {
        println!("╔══════════════════════════════════════════════════════╗");
        println!("║         LFI INTELLIGENCE BENCHMARK REPORT           ║");
        println!("╠══════════════════════════════════════════════════════╣");
        println!("║ Domains: {:<3}  Examples: {:<4}  Correct: {:<4}       ║",
            report.domains_evaluated, report.total_examples, report.total_correct);
        println!("║ Overall Accuracy:  {:.1}%                            ║", report.overall_accuracy * 100.0);
        println!("║ Weighted Score:    {:.1}%                            ║", report.overall_weighted_score * 100.0);
        println!("║ Strongest: {:<15}  Weakest: {:<15}  ║", report.strongest_domain, report.weakest_domain);
        println!("╠══════════════════════════════════════════════════════╣");
        for ds in &report.domain_scores {
            let bar_len = (ds.accuracy * 20.0) as usize;
            let bar: String = "█".repeat(bar_len);
            let empty: String = "░".repeat(20 - bar_len);
            println!("║ {:<12} {}{} {:>5.1}% ({}/{})   ║",
                ds.domain, bar, empty, ds.accuracy * 100.0, ds.correct, ds.total);
        }
        println!("╚══════════════════════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_runs() -> Result<(), HdcError> {
        let mut knowledge = KnowledgeEngine::new();
        let report = IntelligenceBenchmark::run(&mut knowledge)?;
        assert!(report.domains_evaluated >= 10);
        assert!(report.total_examples >= 90);
        assert!(report.overall_accuracy >= 0.0 && report.overall_accuracy <= 1.0);
        Ok(())
    }

    #[test]
    fn test_benchmark_after_training() -> Result<(), HdcError> {
        let mut knowledge = KnowledgeEngine::new();
        let examples = TrainingDataGenerator::all_examples();

        // Run benchmark twice — second pass should have equal or better accuracy.
        let report1 = IntelligenceBenchmark::run_with_examples(&mut knowledge, &examples)?;
        let report2 = IntelligenceBenchmark::run_with_examples(&mut knowledge, &examples)?;
        assert!(report2.overall_accuracy >= report1.overall_accuracy,
            "Second pass should be at least as accurate: {:.2}% vs {:.2}%",
            report2.overall_accuracy * 100.0, report1.overall_accuracy * 100.0);
        assert!(report2.total_correct >= report1.total_correct);
        Ok(())
    }

    #[test]
    fn test_domain_scores_ordered() -> Result<(), HdcError> {
        let mut knowledge = KnowledgeEngine::new();
        let report = IntelligenceBenchmark::run(&mut knowledge)?;
        // Should be sorted by accuracy descending.
        for i in 1..report.domain_scores.len() {
            assert!(report.domain_scores[i-1].accuracy >= report.domain_scores[i].accuracy);
        }
        Ok(())
    }

    #[test]
    fn test_print_report_no_panic() -> Result<(), HdcError> {
        let mut knowledge = KnowledgeEngine::new();
        let report = IntelligenceBenchmark::run(&mut knowledge)?;
        IntelligenceBenchmark::print_report(&report); // Should not panic.
        Ok(())
    }
}
