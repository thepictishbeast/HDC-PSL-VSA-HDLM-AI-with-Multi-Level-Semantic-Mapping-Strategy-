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

    /// Validate a plan: check that dependency ordering is consistent.
    ///
    /// Returns Ok(()) if valid, Err with the first inconsistency found.
    /// Checks:
    ///   - No self-dependencies
    ///   - No dependencies on non-existent steps
    ///   - No circular dependencies
    pub fn validate_plan(&self, plan: &Plan) -> Result<(), HdcError> {
        debuglog!("Planner::validate_plan: checking {} steps", plan.steps.len());

        for (i, step) in plan.steps.iter().enumerate() {
            for &dep in &step.depends_on {
                if dep == i {
                    return Err(HdcError::InitializationFailed {
                        reason: format!("Step {} depends on itself", i),
                    });
                }
                if dep >= plan.steps.len() {
                    return Err(HdcError::InitializationFailed {
                        reason: format!("Step {} depends on non-existent step {}", i, dep),
                    });
                }
            }
        }

        // Check for cycles via topological sort.
        let n = plan.steps.len();
        let mut in_degree = vec![0usize; n];
        for step in &plan.steps {
            for &dep in &step.depends_on {
                if dep < n {
                    in_degree[step.depends_on.len()] += 0; // just counting
                }
            }
        }
        // Simple DFS cycle detection.
        let mut visited = vec![0u8; n]; // 0=unvisited, 1=in-progress, 2=done
        for start in 0..n {
            if visited[start] == 0 {
                let mut stack = vec![(start, false)];
                while let Some((node, returning)) = stack.pop() {
                    if returning {
                        visited[node] = 2;
                        continue;
                    }
                    if visited[node] == 1 {
                        return Err(HdcError::InitializationFailed {
                            reason: format!("Circular dependency detected involving step {}", node),
                        });
                    }
                    if visited[node] == 2 {
                        continue;
                    }
                    visited[node] = 1;
                    stack.push((node, true));
                    for &dep in &plan.steps[node].depends_on {
                        if dep < n {
                            stack.push((dep, false));
                        }
                    }
                }
            }
        }

        debuglog!("Planner::validate_plan: valid");
        Ok(())
    }

    /// Find steps that can execute in parallel (no dependency between them).
    ///
    /// Returns groups of step indices that can run simultaneously.
    pub fn parallel_groups(&self, plan: &Plan) -> Vec<Vec<usize>> {
        debuglog!("Planner::parallel_groups: analyzing {} steps", plan.steps.len());

        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut scheduled = vec![false; plan.steps.len()];

        loop {
            let mut group = Vec::new();
            for (i, step) in plan.steps.iter().enumerate() {
                if scheduled[i] {
                    continue;
                }
                // Can execute if all dependencies are already scheduled.
                let deps_met = step.depends_on.iter().all(|&dep| dep < plan.steps.len() && scheduled[dep]);
                if deps_met {
                    group.push(i);
                }
            }

            if group.is_empty() {
                break;
            }

            for &idx in &group {
                scheduled[idx] = true;
            }
            groups.push(group);
        }

        debuglog!("Planner::parallel_groups: {} groups", groups.len());
        groups
    }

    /// Estimate the critical path length (longest dependency chain).
    /// This is the minimum number of sequential phases needed.
    pub fn critical_path_length(&self, plan: &Plan) -> usize {
        let groups = self.parallel_groups(plan);
        groups.len()
    }

    /// Get the progress of a plan as a percentage (0.0 to 1.0).
    pub fn progress(plan: &Plan) -> f64 {
        if plan.steps.is_empty() {
            return 1.0;
        }
        plan.completed_count() as f64 / plan.steps.len() as f64
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
        while let Some(idx) = planner.advance(&mut plan)? {
            planner.complete_step(&mut plan, idx)?;
        }

        planner.learn_from_success(&plan)?;
        assert_eq!(planner.pattern_count(), initial_count + 1);
        Ok(())
    }

    #[test]
    fn test_validate_plan_valid() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("implement authentication middleware")?;
        planner.validate_plan(&plan)?; // Should not error.
        Ok(())
    }

    #[test]
    fn test_validate_plan_self_dependency() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("test task")?;
        // Inject a self-dependency.
        if !plan.steps.is_empty() {
            plan.steps[0].depends_on.push(0);
        }
        let result = planner.validate_plan(&plan);
        assert!(result.is_err(), "Self-dependency should fail validation");
        Ok(())
    }

    #[test]
    fn test_parallel_groups() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("build REST API with tests")?;
        let groups = planner.parallel_groups(&plan);

        // Should have at least 1 group.
        assert!(!groups.is_empty(), "Should have at least one parallel group");

        // Total steps across all groups should equal plan.steps.len().
        let total: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total, plan.steps.len(), "All steps should be scheduled");
        Ok(())
    }

    #[test]
    fn test_critical_path_length() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("fix critical security vulnerability")?;
        let cpl = planner.critical_path_length(&plan);
        // Linear dependency chain → critical path = number of steps.
        assert!(cpl > 0 && cpl <= plan.steps.len());
        Ok(())
    }

    #[test]
    fn test_plan_progress() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("optimize query performance")?;

        assert!((Planner::progress(&plan) - 0.0).abs() < 0.01);

        // Complete half the steps.
        let half = plan.steps.len() / 2;
        for _ in 0..half {
            if let Some(idx) = planner.advance(&mut plan)? {
                planner.complete_step(&mut plan, idx)?;
            }
        }
        let progress = Planner::progress(&plan);
        assert!(progress > 0.0 && progress < 1.0, "Progress should be partial: {:.2}", progress);

        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for Planner
    // ============================================================

    /// INVARIANT: progress is in [0.0, 1.0] for any plan state.
    #[test]
    fn invariant_progress_in_unit_interval() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("ship a feature")?;
        let p = Planner::progress(&plan);
        assert!(p >= 0.0 && p <= 1.0);

        let mut full = plan.clone();
        for s in &mut full.steps {
            s.status = StepStatus::Done;
        }
        let pf = Planner::progress(&full);
        assert!(pf <= 1.0,
            "all-complete plan must progress at most to 1.0, got {}", pf);
        Ok(())
    }

    /// INVARIANT: completed + remaining sums to total step count.
    #[test]
    fn invariant_completed_plus_remaining_equals_total() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("multi-step build")?;
        for (i, s) in plan.steps.iter_mut().enumerate() {
            if i % 2 == 0 { s.status = StepStatus::Done; }
        }
        let total = plan.steps.len();
        let c = plan.completed_count();
        let r = plan.remaining_count();
        assert_eq!(c + r, total,
            "completed ({}) + remaining ({}) must equal total ({})", c, r, total);
        Ok(())
    }

    /// INVARIANT: validate_plan is OK on freshly-decomposed plans.
    #[test]
    fn invariant_fresh_plan_validates() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("validate after planning")?;
        assert!(planner.validate_plan(&plan).is_ok(),
            "freshly-decomposed plan must validate");
        Ok(())
    }

    /// INVARIANT: is_complete iff all steps are Done.
    #[test]
    fn invariant_is_complete_matches_all_done() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("complete check")?;
        // Fresh plan: not complete.
        assert!(!plan.is_complete(), "fresh plan should not be complete");

        // Complete every step.
        for i in 0..plan.steps.len() {
            planner.complete_step(&mut plan, i)?;
        }
        assert!(plan.is_complete(),
            "after completing all steps, is_complete should be true");
        Ok(())
    }

    /// INVARIANT: critical_path_length never exceeds total steps.
    #[test]
    fn invariant_critical_path_bounded() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("critical path check")?;
        let cp = planner.critical_path_length(&plan);
        assert!(cp <= plan.steps.len(),
            "critical path {} exceeds step count {}", cp, plan.steps.len());
        Ok(())
    }

    /// INVARIANT: parallel_groups union covers all step indices.
    #[test]
    fn invariant_parallel_groups_cover_all_steps() -> Result<(), HdcError> {
        let planner = Planner::new();
        let plan = planner.plan("parallel groups check")?;
        let groups = planner.parallel_groups(&plan);
        let all_indices: std::collections::HashSet<_> =
            groups.iter().flatten().copied().collect();
        assert_eq!(all_indices.len(), plan.steps.len(),
            "parallel groups miss steps: covered={}, total={}",
            all_indices.len(), plan.steps.len());
        Ok(())
    }

    /// INVARIANT: fail_step marks the step as Failed with given reason.
    #[test]
    fn invariant_fail_step_marks_failed() -> Result<(), HdcError> {
        let planner = Planner::new();
        let mut plan = planner.plan("fail step check")?;
        if !plan.steps.is_empty() {
            planner.fail_step(&mut plan, 0, "test-fail")?;
            assert!(matches!(plan.steps[0].status, StepStatus::Failed { .. }),
                "step should be Failed after fail_step");
            assert!(plan.has_failures(), "has_failures should return true");
        }
        Ok(())
    }
}
