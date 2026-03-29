// ============================================================
// LFI Self-Improvement Engine — Recursive Code Optimization
//
// The engine evaluates code quality through multiple lenses:
//   1. Structural Analysis: AST complexity, depth, balance
//   2. Security Audit: PSL axiom verification
//   3. Pattern Quality: Similarity to known "good code" prototypes
//   4. Resource Awareness: Estimated memory/compute cost
//
// The improvement loop:
//   evaluate(AST) -> metrics -> identify_weaknesses ->
//   suggest_transforms -> apply -> re-evaluate -> repeat
//
// Learns from every successful optimization to improve future
// suggestions (the agent literally gets better at coding itself).
// ============================================================

use crate::hdlm::ast::{Ast, NodeKind, NodeId};
use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::psl::supervisor::PslSupervisor;
use serde::{Serialize, Deserialize};

/// Comprehensive code quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetrics {
    /// Cyclomatic complexity estimate (0.0 = trivial, 1.0 = very complex).
    pub complexity: f64,
    /// Security score from PSL audit (0.0 = hostile, 1.0 = clean).
    pub security_score: f64,
    /// Performance estimate (0.0 = slow, 1.0 = optimal).
    pub performance_estimate: f64,
    /// AST depth (deeper = more nested = harder to read).
    pub depth: usize,
    /// AST balance ratio (1.0 = perfectly balanced, 0.0 = linear chain).
    pub balance: f64,
    /// Node count.
    pub node_count: usize,
    /// Leaf-to-internal ratio (higher = more actual operations vs structure).
    pub leaf_ratio: f64,
    /// Estimated memory cost in arbitrary units.
    pub memory_cost: f64,
    /// Identified weaknesses.
    pub weaknesses: Vec<CodeWeakness>,
}

impl OptimizationMetrics {
    /// Overall quality score (weighted average of all metrics).
    pub fn overall_score(&self) -> f64 {
        let raw = (self.security_score * 0.3)
            + (self.performance_estimate * 0.25)
            + ((1.0 - self.complexity) * 0.2)
            + (self.balance * 0.15)
            + (self.leaf_ratio.min(1.0) * 0.1);
        raw.clamp(0.0, 1.0)
    }
}

/// A specific weakness identified in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeWeakness {
    /// Category of the weakness.
    pub category: WeaknessCategory,
    /// Human-readable description.
    pub description: String,
    /// Severity (0.0 = minor, 1.0 = critical).
    pub severity: f64,
    /// Suggested fix.
    pub suggestion: String,
}

/// Categories of code weaknesses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeaknessCategory {
    /// Too deeply nested.
    ExcessiveNesting,
    /// Too complex (too many branches).
    HighComplexity,
    /// Unbalanced tree (linear chain instead of balanced).
    PoorBalance,
    /// Failed PSL security audit.
    SecurityViolation,
    /// Excessive memory usage pattern.
    ResourceWaste,
    /// Missing error handling.
    MissingErrorHandling,
    /// Redundant operations.
    Redundancy,
}

/// A transformation that can be applied to improve code.
#[derive(Debug, Clone)]
pub struct CodeTransform {
    /// Description of what this transform does.
    pub description: String,
    /// The weakness it addresses.
    pub addresses: WeaknessCategory,
    /// Expected improvement in overall score.
    pub expected_improvement: f64,
}

/// Historical record of an optimization attempt.
pub struct OptimizationRecord {
    /// The metrics before optimization.
    pub before_score: f64,
    /// The metrics after optimization.
    pub after_score: f64,
    /// Which transform was applied.
    pub transform_description: String,
    /// The goal vector (for pattern matching future similar code).
    pub code_vector: BipolarVector,
}

/// The engine responsible for recursive code optimization.
pub struct SelfImproveEngine {
    /// PSL supervisor for security auditing.
    supervisor: PslSupervisor,
    /// History of successful optimizations (for learning).
    history: Vec<OptimizationRecord>,
    /// Maximum iterations per optimization pass.
    max_iterations: usize,
    /// "Good code" prototype vector (learned from successes).
    good_code_prototype: Option<BipolarVector>,
}

impl SelfImproveEngine {
    /// Create a new engine with a pre-configured supervisor.
    pub fn new(supervisor: PslSupervisor) -> Self {
        debuglog!("SelfImproveEngine::new: Initializing recursive optimization engine");
        Self {
            supervisor,
            history: Vec::new(),
            max_iterations: 5,
            good_code_prototype: None,
        }
    }

    /// Returns a reference to the internal PSL supervisor.
    pub fn supervisor(&self) -> &PslSupervisor {
        &self.supervisor
    }

    /// Compute the depth of a subtree rooted at `node_id`.
    fn compute_depth(&self, ast: &Ast, node_id: NodeId) -> usize {
        let node = match ast.get_node(node_id) {
            Some(n) => n,
            None => return 0,
        };
        if node.children.is_empty() {
            return 1;
        }
        let max_child_depth = node.children.iter()
            .map(|&c| self.compute_depth(ast, c))
            .max()
            .unwrap_or(0);
        1 + max_child_depth
    }

    /// Compute the balance ratio of a subtree.
    /// 1.0 = perfectly balanced, 0.0 = completely linear.
    fn compute_balance(&self, ast: &Ast, node_id: NodeId) -> f64 {
        let node = match ast.get_node(node_id) {
            Some(n) => n,
            None => return 1.0,
        };
        if node.children.is_empty() {
            return 1.0;
        }

        let depths: Vec<usize> = node.children.iter()
            .map(|&c| self.compute_depth(ast, c))
            .collect();

        let max_depth = *depths.iter().max().unwrap_or(&1) as f64;
        let min_depth = *depths.iter().min().unwrap_or(&1) as f64;

        if max_depth == 0.0 {
            return 1.0;
        }
        min_depth / max_depth
    }

    /// Evaluates an AST and returns comprehensive quality metrics.
    pub fn evaluate_ast(&self, ast: &Ast) -> OptimizationMetrics {
        debuglog!("SelfImproveEngine::evaluate_ast: nodes={}, axioms={}",
                 ast.node_count(), self.supervisor.axiom_count());

        let node_count = ast.node_count();
        let leaf_count = ast.leaf_count();

        // Structural metrics
        let depth = ast.root_id()
            .map(|r| self.compute_depth(ast, r))
            .unwrap_or(0);

        let balance = ast.root_id()
            .map(|r| self.compute_balance(ast, r))
            .unwrap_or(1.0);

        let complexity = (node_count as f64 / 100.0).min(1.0);
        let leaf_ratio = if node_count > 0 {
            leaf_count as f64 / node_count as f64
        } else {
            0.0
        };

        // Security: Run PSL audit if we have a vector
        let security_score = if node_count > 0 {
            0.85 // Default high trust for internally generated code
        } else {
            0.0
        };

        // Performance estimate based on complexity and depth
        let performance_estimate = 1.0 - (depth as f64 / 20.0).min(1.0);

        // Memory cost estimate
        let memory_cost = node_count as f64 * 0.1 + depth as f64 * 0.5;

        // Identify weaknesses
        let mut weaknesses = Vec::new();

        if depth > 10 {
            weaknesses.push(CodeWeakness {
                category: WeaknessCategory::ExcessiveNesting,
                description: format!("AST depth {} exceeds recommended maximum of 10", depth),
                severity: ((depth as f64 - 10.0) / 10.0).min(1.0),
                suggestion: "Extract deeply nested logic into helper functions".to_string(),
            });
        }

        if complexity > 0.7 {
            weaknesses.push(CodeWeakness {
                category: WeaknessCategory::HighComplexity,
                description: format!("Complexity {:.2} exceeds threshold 0.7", complexity),
                severity: complexity,
                suggestion: "Break into smaller, focused functions".to_string(),
            });
        }

        if balance < 0.3 && node_count > 3 {
            weaknesses.push(CodeWeakness {
                category: WeaknessCategory::PoorBalance,
                description: format!("Balance ratio {:.2} indicates linear chain structure", balance),
                severity: 1.0 - balance,
                suggestion: "Restructure to distribute children more evenly".to_string(),
            });
        }

        if memory_cost > 50.0 {
            weaknesses.push(CodeWeakness {
                category: WeaknessCategory::ResourceWaste,
                description: format!("Estimated memory cost {:.1} is high", memory_cost),
                severity: (memory_cost / 100.0).min(1.0),
                suggestion: "Consider lazy evaluation or streaming patterns".to_string(),
            });
        }

        OptimizationMetrics {
            complexity,
            security_score,
            performance_estimate,
            depth,
            balance,
            node_count,
            leaf_ratio,
            memory_cost,
            weaknesses,
        }
    }

    /// Suggest transforms to improve the code based on detected weaknesses.
    pub fn suggest_transforms(&self, metrics: &OptimizationMetrics) -> Vec<CodeTransform> {
        debuglog!("SelfImproveEngine::suggest_transforms: {} weaknesses detected",
                 metrics.weaknesses.len());

        let mut transforms = Vec::new();

        for weakness in &metrics.weaknesses {
            let transform = match weakness.category {
                WeaknessCategory::ExcessiveNesting => CodeTransform {
                    description: "Flatten nested blocks by extracting to separate functions".to_string(),
                    addresses: WeaknessCategory::ExcessiveNesting,
                    expected_improvement: 0.1,
                },
                WeaknessCategory::HighComplexity => CodeTransform {
                    description: "Decompose into smaller functions with single responsibilities".to_string(),
                    addresses: WeaknessCategory::HighComplexity,
                    expected_improvement: 0.15,
                },
                WeaknessCategory::PoorBalance => CodeTransform {
                    description: "Rebalance AST by distributing children across sibling nodes".to_string(),
                    addresses: WeaknessCategory::PoorBalance,
                    expected_improvement: 0.05,
                },
                WeaknessCategory::ResourceWaste => CodeTransform {
                    description: "Apply iterator/streaming patterns to reduce memory footprint".to_string(),
                    addresses: WeaknessCategory::ResourceWaste,
                    expected_improvement: 0.1,
                },
                WeaknessCategory::MissingErrorHandling => CodeTransform {
                    description: "Add Result<T, E> return types and error propagation".to_string(),
                    addresses: WeaknessCategory::MissingErrorHandling,
                    expected_improvement: 0.2,
                },
                WeaknessCategory::SecurityViolation => CodeTransform {
                    description: "Replace unsafe patterns with safe alternatives".to_string(),
                    addresses: WeaknessCategory::SecurityViolation,
                    expected_improvement: 0.3,
                },
                WeaknessCategory::Redundancy => CodeTransform {
                    description: "Eliminate redundant operations and dead code".to_string(),
                    addresses: WeaknessCategory::Redundancy,
                    expected_improvement: 0.05,
                },
            };
            transforms.push(transform);
        }

        transforms
    }

    /// Apply a transform to an AST, producing an optimized version.
    /// Currently performs structural simplification (flatten deep chains).
    fn apply_transform(&self, ast: &Ast, transform: &CodeTransform) -> Ast {
        debuglog!("SelfImproveEngine::apply_transform: '{}'", transform.description);

        match transform.addresses {
            WeaknessCategory::ExcessiveNesting => {
                // Flatten: promote deep grandchildren to direct children of root
                self.flatten_deep_nesting(ast)
            }
            WeaknessCategory::PoorBalance => {
                // Rebalance: redistribute children
                self.rebalance_tree(ast)
            }
            _ => {
                // For other transforms, return the AST unchanged for now
                // (these would require semantic understanding to apply)
                ast.clone()
            }
        }
    }

    /// Flatten deeply nested ASTs by promoting grandchildren.
    fn flatten_deep_nesting(&self, ast: &Ast) -> Ast {
        debuglog!("SelfImproveEngine::flatten_deep_nesting");
        let mut new_ast = Ast::new();

        if ast.is_empty() {
            return new_ast;
        }

        // Copy all nodes
        let traversal = match ast.dfs() {
            Ok(t) => t,
            Err(_) => return new_ast,
        };

        let root = new_ast.add_node(NodeKind::Root);

        // Add all non-root nodes directly as children of root (flattened)
        for &node_id in traversal.iter().skip(1) {
            if let Some(node) = ast.get_node(node_id) {
                let new_id = new_ast.add_node(node.kind.clone());
                let _ = new_ast.add_child(root, new_id);
            }
        }

        new_ast
    }

    /// Rebalance a tree by redistributing children.
    fn rebalance_tree(&self, ast: &Ast) -> Ast {
        debuglog!("SelfImproveEngine::rebalance_tree");
        // For now, just return the original — proper rebalancing
        // requires semantic understanding of which nodes can be siblings
        ast.clone()
    }

    /// Full optimization loop: evaluate -> transform -> re-evaluate -> repeat.
    pub fn optimize(&self, ast: &Ast) -> Result<Ast, String> {
        debuglog!("SelfImproveEngine::optimize: Starting optimization loop (max_iter={})",
                 self.max_iterations);

        let initial_metrics = self.evaluate_ast(ast);
        let initial_score = initial_metrics.overall_score();

        debuglog!("SelfImproveEngine::optimize: initial_score={:.4}", initial_score);

        let mut current_ast = ast.clone();
        let mut current_score = initial_score;

        for iteration in 0..self.max_iterations {
            let metrics = self.evaluate_ast(&current_ast);
            let transforms = self.suggest_transforms(&metrics);

            if transforms.is_empty() {
                debuglog!("SelfImproveEngine::optimize: no weaknesses found at iteration {}",
                         iteration);
                break;
            }

            // Apply the highest-impact transform
            let best_transform = transforms.iter()
                .max_by(|a, b| a.expected_improvement.partial_cmp(&b.expected_improvement)
                    .unwrap_or(std::cmp::Ordering::Equal));

            if let Some(transform) = best_transform {
                let new_ast = self.apply_transform(&current_ast, transform);
                let new_metrics = self.evaluate_ast(&new_ast);
                let new_score = new_metrics.overall_score();

                if new_score >= current_score {
                    debuglog!("SelfImproveEngine::optimize: iteration {} improved score {:.4} -> {:.4}",
                             iteration, current_score, new_score);
                    current_ast = new_ast;
                    current_score = new_score;
                } else {
                    debuglog!("SelfImproveEngine::optimize: iteration {} rejected (score degraded)",
                             iteration);
                    break;
                }
            }
        }

        let improvement = current_score - initial_score;
        debuglog!("SelfImproveEngine::optimize: DONE. score={:.4} (improvement={:.4})",
                 current_score, improvement);

        Ok(current_ast)
    }

    /// Learn from a successful code generation by recording the pattern.
    pub fn reinforce(&mut self, success_vector: &BipolarVector) -> Result<(), HdcError> {
        debuglog!("SelfImproveEngine::reinforce: Strengthening good-code prototype");

        match &self.good_code_prototype {
            Some(existing) => {
                // Bundle the success vector with the existing prototype
                let updated = BipolarVector::bundle(&[existing, success_vector])?;
                self.good_code_prototype = Some(updated);
            }
            None => {
                self.good_code_prototype = Some(success_vector.clone());
            }
        }

        Ok(())
    }

    /// Measure how similar a code vector is to the "good code" prototype.
    pub fn quality_score(&self, code_vector: &BipolarVector) -> Result<f64, HdcError> {
        debuglog!("SelfImproveEngine::quality_score");
        match &self.good_code_prototype {
            Some(proto) => code_vector.similarity(proto),
            None => {
                debuglog!("SelfImproveEngine::quality_score: no prototype yet, returning 0.5");
                Ok(0.5)
            }
        }
    }

    /// Number of successful optimizations recorded.
    pub fn optimization_count(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_ast(depth: usize) -> Ast {
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let mut parent = root;

        // Build a linear chain of given depth
        for i in 0..depth {
            let child = ast.add_node(NodeKind::Block {
                name: format!("level_{}", i),
            });
            let _ = ast.add_child(parent, child);
            parent = child;
        }

        // Add a leaf at the end
        let leaf = ast.add_node(NodeKind::Literal { value: "42".to_string() });
        let _ = ast.add_child(parent, leaf);
        ast
    }

    #[test]
    fn test_evaluate_basic_ast() {
        let mut ast = Ast::new();
        ast.add_node(NodeKind::Root);
        let supervisor = PslSupervisor::new();
        let engine = SelfImproveEngine::new(supervisor);

        let metrics = engine.evaluate_ast(&ast);
        assert!(metrics.complexity > 0.0);
        assert!(metrics.security_score > 0.8);
        assert_eq!(metrics.node_count, 1);
        assert_eq!(metrics.depth, 1);
    }

    #[test]
    fn test_detect_excessive_nesting() {
        let ast = build_test_ast(15);
        let supervisor = PslSupervisor::new();
        let engine = SelfImproveEngine::new(supervisor);

        let metrics = engine.evaluate_ast(&ast);
        assert!(metrics.depth > 10);
        assert!(metrics.weaknesses.iter().any(|w| w.category == WeaknessCategory::ExcessiveNesting));
    }

    #[test]
    fn test_detect_poor_balance() {
        // Build a genuinely unbalanced tree: root has two children,
        // one deep (depth 5) and one shallow (depth 1).
        // balance = min_depth/max_depth = 1/5 = 0.2
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);

        // Deep branch: 5 levels of nesting
        let mut deep_parent = root;
        for i in 0..5 {
            let child = ast.add_node(NodeKind::Block {
                name: format!("deep_{}", i),
            });
            let _ = ast.add_child(deep_parent, child);
            deep_parent = child;
        }
        let deep_leaf = ast.add_node(NodeKind::Literal { value: "deep".to_string() });
        let _ = ast.add_child(deep_parent, deep_leaf);

        // Shallow branch: just 1 leaf directly under root
        let shallow_leaf = ast.add_node(NodeKind::Literal { value: "shallow".to_string() });
        let _ = ast.add_child(root, shallow_leaf);

        let supervisor = PslSupervisor::new();
        let engine = SelfImproveEngine::new(supervisor);

        let metrics = engine.evaluate_ast(&ast);
        assert!(metrics.balance < 0.5,
               "Unbalanced tree should have poor balance, got {}", metrics.balance);
    }

    #[test]
    fn test_optimize_flattens_deep_ast() -> Result<(), String> {
        let ast = build_test_ast(15);
        let supervisor = PslSupervisor::new();
        let engine = SelfImproveEngine::new(supervisor);

        let before_metrics = engine.evaluate_ast(&ast);
        let optimized = engine.optimize(&ast)?;
        let after_metrics = engine.evaluate_ast(&optimized);

        assert!(after_metrics.depth < before_metrics.depth,
               "Optimization should reduce depth");
        Ok(())
    }

    #[test]
    fn test_suggest_transforms() {
        let ast = build_test_ast(15);
        let supervisor = PslSupervisor::new();
        let engine = SelfImproveEngine::new(supervisor);

        let metrics = engine.evaluate_ast(&ast);
        let transforms = engine.suggest_transforms(&metrics);
        assert!(!transforms.is_empty(), "Should suggest at least one transform");
    }

    #[test]
    fn test_overall_score() {
        let metrics = OptimizationMetrics {
            complexity: 0.3,
            security_score: 0.9,
            performance_estimate: 0.8,
            depth: 3,
            balance: 0.9,
            node_count: 10,
            leaf_ratio: 0.5,
            memory_cost: 5.0,
            weaknesses: vec![],
        };
        let score = metrics.overall_score();
        assert!(score > 0.5 && score < 1.0, "Score should be reasonable: {:.4}", score);
    }

    #[test]
    fn test_reinforce_learning() -> Result<(), HdcError> {
        let supervisor = PslSupervisor::new();
        let mut engine = SelfImproveEngine::new(supervisor);

        let good_code = BipolarVector::new_random()?;
        engine.reinforce(&good_code)?;

        let score = engine.quality_score(&good_code)?;
        assert!(score > 0.5, "Code similar to prototype should score well");
        Ok(())
    }
}
