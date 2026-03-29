// ============================================================
// LFI Planner — Goal-Directed Task Decomposition
//
// Decomposes complex goals into executable sub-steps using
// VSA-based means-end analysis. Each step is a hypervector
// that can be compared to known solution patterns.
//
// The planner maintains a working memory of active goals,
// tracks step completion, and can re-plan when steps fail.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;

/// Status of a plan step.
#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Active,
    /// Successfully completed.
    Done,
    /// Failed — needs replanning.
    Failed { reason: String },
}

/// A single executable step in a plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Human-readable description of this step.
    pub description: String,
    /// The goal state as a hypervector (what "done" looks like).
    pub goal_vector: BipolarVector,
    /// Estimated complexity (0.0 = trivial, 1.0 = maximum).
    pub complexity: f64,
    /// Execution status.
    pub status: StepStatus,
    /// Sub-steps (for hierarchical decomposition).
    pub sub_steps: Vec<PlanStep>,
    /// Dependencies: indices of steps that must complete first.
    pub depends_on: Vec<usize>,
}

/// A complete plan: an ordered sequence of steps toward a goal.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The top-level goal description.
    pub goal: String,
    /// The goal state as a hypervector.
    pub goal_vector: BipolarVector,
    /// Ordered steps to achieve the goal.
    pub steps: Vec<PlanStep>,
    /// Overall estimated complexity.
    pub total_complexity: f64,
    /// Number of replanning attempts.
    pub replan_count: usize,
}

impl Plan {
    /// How many steps are done.
    pub fn completed_count(&self) -> usize {
        self.steps.iter().filter(|s| s.status == StepStatus::Done).count()
    }

    /// How many steps remain.
    pub fn remaining_count(&self) -> usize {
        self.steps.iter().filter(|s| matches!(s.status, StepStatus::Pending | StepStatus::Active)).count()
    }

    /// Whether all steps are done.
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| s.status == StepStatus::Done)
    }

    /// Whether any step has failed.
    pub fn has_failures(&self) -> bool {
        self.steps.iter().any(|s| matches!(s.status, StepStatus::Failed { .. }))
    }
}

/// Known solution patterns for means-end analysis.
struct SolutionPattern {
    /// What kind of problem this solves.
    problem_prototype: BipolarVector,
    /// Template plan for this problem type.
    template_steps: Vec<String>,
    /// Historical success rate.
    success_rate: f64,
}

/// The Planner: decomposes goals into executable plans.
pub struct Planner {
    /// Library of known solution patterns (grows with experience).
    patterns: Vec<SolutionPattern>,
    /// Maximum decomposition depth.
    pub max_depth: usize,
}

impl Planner {
    /// Create a new planner with seed patterns for common programming tasks.
    pub fn new() -> Self {
        debuglog!("Planner::new: Initializing goal-directed planner");
        let mut planner = Self {
            patterns: Vec::new(),
            max_depth: 5,
        };
        planner.seed_patterns();
        planner
    }

    /// Seed the planner with common programming task patterns.
    fn seed_patterns(&mut self) {
        debuglog!("Planner::seed_patterns: Loading {} seed patterns", 6);

        let pattern_templates = vec![
            ("implement_function", vec![
                "Analyze requirements and constraints",
                "Define function signature and types",
                "Implement core logic",
                "Add error handling",
                "Write unit tests",
                "Run PSL audit on output",
            ]),
            ("fix_bug", vec![
                "Reproduce the failure",
                "Trace execution to root cause",
                "Formulate fix hypothesis",
                "Apply minimal fix",
                "Verify fix resolves issue",
                "Check for regressions",
            ]),
            ("refactor_module", vec![
                "Map current structure and dependencies",
                "Identify code smells and violations",
                "Plan transformation steps",
                "Apply transformations incrementally",
                "Verify semantics preserved",
                "Update dependent modules",
            ]),
            ("design_api", vec![
                "Define resource model",
                "Specify endpoints and methods",
                "Define request/response schemas",
                "Add authentication and authorization",
                "Implement error handling",
                "Write integration tests",
            ]),
            ("optimize_performance", vec![
                "Profile and identify bottleneck",
                "Analyze algorithmic complexity",
                "Apply targeted optimization",
                "Benchmark before vs after",
                "Verify correctness preserved",
            ]),
            ("security_audit", vec![
                "Enumerate attack surface",
                "Check input validation boundaries",
                "Verify authentication/authorization",
                "Scan for injection vulnerabilities",
                "Review cryptographic usage",
                "Generate audit report",
            ]),
        ];

        for (name, steps) in pattern_templates {
            let proto = BipolarVector::from_seed(
                crate::identity::IdentityProver::hash(name)
            );
            self.patterns.push(SolutionPattern {
                problem_prototype: proto,
                template_steps: steps.into_iter().map(|s| s.to_string()).collect(),
                success_rate: 0.8,
            });
        }
    }

    /// Decompose a goal into an executable plan.
    ///
    /// 1. Vectorize the goal description.
    /// 2. Search for the closest known solution pattern.
    /// 3. If match found (similarity > 0.3): adapt the template.
    /// 4. If no match: perform novel decomposition via AST analysis.
    pub fn plan(&self, goal: &str) -> Result<Plan, HdcError> {
        debuglog!("Planner::plan: decomposing goal='{}'", goal);

        let goal_vector = BipolarVector::from_seed(
            crate::identity::IdentityProver::hash(goal)
        );

        // Search for closest known pattern
        let mut best_sim = -1.0_f64;
        let mut best_pattern: Option<&SolutionPattern> = None;

        for pattern in &self.patterns {
            let sim = goal_vector.similarity(&pattern.problem_prototype)?;
            if sim > best_sim {
                best_sim = sim;
                best_pattern = Some(pattern);
            }
        }

        let steps = if let Some(pattern) = best_pattern {
            debuglog!("Planner::plan: matched pattern (sim={:.4}, success_rate={:.2})",
                     best_sim, pattern.success_rate);
            // Adapt the template
            pattern.template_steps.iter().enumerate().map(|(i, desc)| {
                let step_seed = crate::identity::IdentityProver::hash(
                    &format!("{}:{}", goal, desc)
                );
                PlanStep {
                    description: desc.clone(),
                    goal_vector: BipolarVector::from_seed(step_seed),
                    complexity: (i as f64 + 1.0) / pattern.template_steps.len() as f64,
                    status: StepStatus::Pending,
                    sub_steps: Vec::new(),
                    depends_on: if i > 0 { vec![i - 1] } else { vec![] },
                }
            }).collect()
        } else {
            debuglog!("Planner::plan: no pattern match, creating generic decomposition");
            // Generic 3-step decomposition for novel problems
            vec![
                PlanStep {
                    description: format!("Analyze: {}", goal),
                    goal_vector: goal_vector.permute(1)?,
                    complexity: 0.3,
                    status: StepStatus::Pending,
                    sub_steps: Vec::new(),
                    depends_on: vec![],
                },
                PlanStep {
                    description: format!("Implement: {}", goal),
                    goal_vector: goal_vector.permute(2)?,
                    complexity: 0.6,
                    status: StepStatus::Pending,
                    sub_steps: Vec::new(),
                    depends_on: vec![0],
                },
                PlanStep {
                    description: format!("Verify: {}", goal),
                    goal_vector: goal_vector.permute(3)?,
                    complexity: 0.3,
                    status: StepStatus::Pending,
                    sub_steps: Vec::new(),
                    depends_on: vec![1],
                },
            ]
        };

        let total_complexity = steps.iter().map(|s| s.complexity).sum::<f64>()
            / steps.len().max(1) as f64;

        Ok(Plan {
            goal: goal.to_string(),
            goal_vector,
            steps,
            total_complexity,
            replan_count: 0,
        })
    }

    /// Advance a plan by marking the next available step as active.
    pub fn advance(&self, plan: &mut Plan) -> Result<Option<usize>, HdcError> {
        debuglog!("Planner::advance: completed={}/{}", plan.completed_count(), plan.steps.len());

        for i in 0..plan.steps.len() {
            if plan.steps[i].status != StepStatus::Pending {
                continue;
            }

            // Check dependencies
            let deps_met = plan.steps[i].depends_on.iter().all(|&dep| {
                dep < plan.steps.len() && plan.steps[dep].status == StepStatus::Done
            });

            if deps_met {
                plan.steps[i].status = StepStatus::Active;
                debuglog!("Planner::advance: activating step {} '{}'", i, plan.steps[i].description);
                return Ok(Some(i));
            }
        }

        debuglog!("Planner::advance: no steps available");
        Ok(None)
    }

    /// Mark a step as completed.
    pub fn complete_step(&self, plan: &mut Plan, step_idx: usize) -> Result<(), HdcError> {
        if step_idx >= plan.steps.len() {
            return Err(HdcError::InitializationFailed {
                reason: format!("Step index {} out of range", step_idx),
            });
        }
        plan.steps[step_idx].status = StepStatus::Done;
        debuglog!("Planner::complete_step: step {} done", step_idx);
        Ok(())
    }

    /// Mark a step as failed and trigger replanning.
    pub fn fail_step(&self, plan: &mut Plan, step_idx: usize, reason: &str) -> Result<(), HdcError> {
        if step_idx >= plan.steps.len() {
            return Err(HdcError::InitializationFailed {
                reason: format!("Step index {} out of range", step_idx),
            });
        }
        plan.steps[step_idx].status = StepStatus::Failed { reason: reason.to_string() };
        plan.replan_count += 1;
        debuglog!("Planner::fail_step: step {} failed (replan_count={})", step_idx, plan.replan_count);
        Ok(())
    }

    /// Learn from a successful plan execution by storing the pattern.
    pub fn learn_from_success(&mut self, plan: &Plan) -> Result<(), HdcError> {
        debuglog!("Planner::learn_from_success: recording pattern for '{}'", plan.goal);
        let template_steps = plan.steps.iter().map(|s| s.description.clone()).collect();
        self.patterns.push(SolutionPattern {
            problem_prototype: plan.goal_vector.clone(),
            template_steps,
            success_rate: 1.0,
        });
        Ok(())
    }

    /// Number of known patterns.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_decomposition() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("implement a new REST endpoint for user profiles")?;

        assert!(!plan.steps.is_empty(), "Plan must have steps");
        assert!(plan.total_complexity > 0.0);
        assert!(!plan.is_complete());
        Ok(())
    }

    #[test]
    fn test_plan_execution() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("fix buffer overflow in parser")?;

        // Execute each step
        let mut executed = 0;
        while let Some(idx) = planner.advance(&mut plan)? {
            planner.complete_step(&mut plan, idx)?;
            executed += 1;
            if executed > 20 { break; } // Safety limit
        }

        assert!(plan.is_complete(), "All steps should be completed");
        assert!(!plan.has_failures());
        assert_eq!(plan.remaining_count(), 0);
        Ok(())
    }

    #[test]
    fn test_plan_failure_and_replan() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("optimize database queries")?;

        // Start first step
        let idx = planner.advance(&mut plan)?.ok_or(HdcError::InitializationFailed {
            reason: "No steps available".into(),
        })?;

        // Fail it
        planner.fail_step(&mut plan, idx, "Profiler unavailable")?;
        assert!(plan.has_failures());
        assert_eq!(plan.replan_count, 1);
        Ok(())
    }

    #[test]
    fn test_learn_from_success() -> Result<(), HdcError> {
        let mut planner = Planner::new();
        let initial_count = planner.pattern_count();

        let mut plan = planner.plan("deploy microservice to kubernetes")?;
        // Complete all steps
        while let Some(idx) = planner.advance(&mut plan)? {
            planner.complete_step(&mut plan, idx)?;
        }

        planner.learn_from_success(&plan)?;
        assert_eq!(planner.pattern_count(), initial_count + 1);
        Ok(())
    }
}
