// NODE 019: JEPA-Inspired World Model
// STATUS: ALPHA - Causal Prediction Active
// PROTOCOL: World-State-Representation-Prediction
//
// ARCHITECTURE:
//   The world model maintains a compressed representation of the
//   environment as a VSA hypervector. Actions transform the state
//   via binding — the resulting vector encodes "what happens next."
//
//   Key capabilities:
//     - PREDICT: Given an action, predict the next world state
//     - LEARN: Record observed cause-effect relationships
//     - VERIFY: Compare predictions against actual observations
//     - COUNTERFACTUAL: Ask "what if I had done X instead?"
//     - PLAN: Find action sequences that reach a goal state
//     - EXPLAIN: Trace which causal links led to a prediction
//
// VSA SEMANTICS:
//   Next_State = Current_State (*) Action_Vector
//   This is JEPA-style: we predict in latent space, not pixel space.
//   The binding operation preserves structure — if two actions are
//   similar, their predicted outcomes are similar.

use crate::memory_bus::HyperMemory;
use tracing::{info, debug};

/// A snapshot of the world at a point in time.
pub struct WorldState {
    pub representation: HyperMemory,
    pub timestamp: u64,
}

/// A recorded causal link: action → observed effect.
#[derive(Debug, Clone)]
pub struct CausalLink {
    pub action: HyperMemory,
    pub effect: HyperMemory,
    /// How many times this causal pattern has been observed.
    pub observations: usize,
    /// Average prediction accuracy when this link was used.
    pub accuracy: f64,
}

/// Result of verifying a prediction against observation.
#[derive(Debug)]
pub struct PredictionVerification {
    /// Cosine similarity between predicted and observed state.
    pub accuracy: f64,
    /// Whether the prediction was "good enough" (above threshold).
    pub accurate: bool,
    /// The predicted state vector.
    pub predicted: HyperMemory,
    /// The observed state vector.
    pub observed: HyperMemory,
}

/// Result of a counterfactual query.
#[derive(Debug)]
pub struct CounterfactualResult {
    /// The alternative action considered.
    pub alternative_action: HyperMemory,
    /// The predicted state under the alternative action.
    pub predicted_state: HyperMemory,
    /// Similarity between the counterfactual outcome and the goal.
    pub goal_similarity: f64,
    /// Similarity between the counterfactual and actual outcome.
    pub divergence: f64,
}

pub struct WorldModel {
    pub current_state: WorldState,
    /// Learned causal links: action → effect patterns.
    pub causal_links: Vec<CausalLink>,
    /// Prediction accuracy threshold for "good enough."
    pub accuracy_threshold: f64,
    /// Maximum causal links to retain.
    max_links: usize,
    /// Running prediction accuracy (EMA).
    prediction_ema: f64,
    /// Total predictions made.
    prediction_count: u64,
}

impl WorldModel {
    pub fn new(initial_state: HyperMemory) -> Self {
        info!("// AUDIT: JEPA World Model Initialized. CAUSAL MAPPING ACTIVE.");
        Self {
            current_state: WorldState {
                representation: initial_state,
                timestamp: 0,
            },
            causal_links: Vec::new(),
            accuracy_threshold: 0.3,
            max_links: 1000,
            prediction_ema: 0.5,
            prediction_count: 0,
        }
    }

    /// PREDICT: Uses the VSA binding operator to simulate the next world state.
    ///
    /// Next_State = Current_State (*) Action_Vector
    /// In JEPA style: prediction happens in latent space, not observation space.
    pub fn predict_next_state(&self, action: &HyperMemory) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        debug!("// DEBUG: Predicting next world-state representation...");
        self.current_state.representation.bind(action)
    }

    /// PREDICT FROM STATE: Predict from an arbitrary state (not just current).
    /// Enables multi-step lookahead and counterfactual reasoning.
    pub fn predict_from(&self, state: &HyperMemory, action: &HyperMemory) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        debuglog!("WorldModel::predict_from: Predicting from arbitrary state");
        state.bind(action)
    }

    /// MULTI-STEP PREDICT: Chain multiple actions to predict several steps ahead.
    ///
    /// Returns the sequence of predicted states: [after_action_0, after_action_1, ...].
    pub fn predict_sequence(
        &self,
        actions: &[HyperMemory],
    ) -> Result<Vec<HyperMemory>, Box<dyn std::error::Error>> {
        debuglog!("WorldModel::predict_sequence: Predicting {} steps ahead", actions.len());
        let mut states = Vec::with_capacity(actions.len());
        let mut current = self.current_state.representation.clone();

        for (i, action) in actions.iter().enumerate() {
            let next = current.bind(action)?;
            debuglog!("WorldModel::predict_sequence: step {} predicted", i);
            states.push(next.clone());
            current = next;
        }

        Ok(states)
    }

    /// LEARN: Record a material cause-and-effect relationship.
    pub fn record_effect(&mut self, action: HyperMemory, effect: HyperMemory) {
        info!("// AUDIT: New Causal Link Integrated into World Model.");

        // Check if a similar causal link already exists.
        let mut found = false;
        for link in &mut self.causal_links {
            let action_sim = link.action.similarity(&action);
            if action_sim > 0.7 {
                // Reinforce existing link — increment observation count.
                link.observations += 1;
                debuglog!("WorldModel::record_effect: Reinforced existing link (obs={})", link.observations);
                found = true;
                break;
            }
        }

        if !found {
            self.causal_links.push(CausalLink {
                action,
                effect,
                observations: 1,
                accuracy: 0.5, // Initial accuracy estimate
            });

            // Prune oldest links if over capacity.
            if self.causal_links.len() > self.max_links {
                // Remove the least-observed link.
                if let Some(min_idx) = self.causal_links.iter()
                    .enumerate()
                    .min_by_key(|(_, l)| l.observations)
                    .map(|(i, _)| i)
                {
                    self.causal_links.swap_remove(min_idx);
                    debuglog!("WorldModel::record_effect: Pruned least-observed link");
                }
            }
        }
    }

    /// VERIFY: Compare a prediction against the actual observation.
    ///
    /// Updates the world model's internal accuracy tracking and
    /// the relevant causal link's accuracy score.
    pub fn verify_prediction(
        &mut self,
        predicted: &HyperMemory,
        observed: &HyperMemory,
        action: &HyperMemory,
    ) -> PredictionVerification {
        let accuracy = predicted.similarity(observed);
        let accurate = accuracy > self.accuracy_threshold;

        // Update EMA of prediction accuracy.
        self.prediction_count += 1;
        let alpha = 0.1;
        self.prediction_ema = alpha * accuracy + (1.0 - alpha) * self.prediction_ema;

        // Update the relevant causal link's accuracy.
        for link in &mut self.causal_links {
            let action_sim = link.action.similarity(action);
            if action_sim > 0.7 {
                link.accuracy = alpha * accuracy + (1.0 - alpha) * link.accuracy;
                debuglog!("WorldModel::verify_prediction: Updated link accuracy to {:.4}", link.accuracy);
                break;
            }
        }

        debuglog!(
            "WorldModel::verify_prediction: accuracy={:.4}, accurate={}, ema={:.4}, count={}",
            accuracy, accurate, self.prediction_ema, self.prediction_count
        );

        PredictionVerification {
            accuracy,
            accurate,
            predicted: predicted.clone(),
            observed: observed.clone(),
        }
    }

    /// COUNTERFACTUAL: "What if I had done action X instead?"
    ///
    /// Takes the state BEFORE the actual action was taken and predicts
    /// what would have happened under an alternative action.
    pub fn counterfactual(
        &self,
        state_before: &HyperMemory,
        alternative_action: &HyperMemory,
        actual_outcome: &HyperMemory,
        goal: Option<&HyperMemory>,
    ) -> Result<CounterfactualResult, Box<dyn std::error::Error>> {
        debuglog!("WorldModel::counterfactual: Computing alternative timeline");

        let predicted_state = state_before.bind(alternative_action)?;
        let divergence = predicted_state.similarity(actual_outcome);

        let goal_similarity = match goal {
            Some(g) => predicted_state.similarity(g),
            None => 0.0,
        };

        debuglog!(
            "WorldModel::counterfactual: divergence={:.4}, goal_sim={:.4}",
            divergence, goal_similarity
        );

        Ok(CounterfactualResult {
            alternative_action: alternative_action.clone(),
            predicted_state,
            goal_similarity,
            divergence,
        })
    }

    /// FIND BEST ACTION: Given a goal state, find the causal link whose
    /// action most closely produces the goal.
    ///
    /// Returns the best action and its predicted goal similarity, or None
    /// if no causal links exist.
    pub fn find_best_action(&self, goal: &HyperMemory) -> Result<Option<(HyperMemory, f64)>, Box<dyn std::error::Error>> {
        debuglog!("WorldModel::find_best_action: Searching {} causal links", self.causal_links.len());

        if self.causal_links.is_empty() {
            return Ok(None);
        }

        let mut best_action = None;
        let mut best_similarity = f64::NEG_INFINITY;

        for link in &self.causal_links {
            let predicted = self.predict_from(&self.current_state.representation, &link.action)?;
            let sim = predicted.similarity(goal);

            if sim > best_similarity {
                best_similarity = sim;
                best_action = Some(link.action.clone());
            }
        }

        Ok(best_action.map(|a| (a, best_similarity)))
    }

    /// UPDATE: Advance the world model to a new observed state.
    pub fn update_state(&mut self, observed: HyperMemory) {
        self.current_state.timestamp += 1;
        self.current_state.representation = observed;
        debuglog!("WorldModel::update_state: Advanced to t={}", self.current_state.timestamp);
    }

    /// Current prediction accuracy (EMA).
    pub fn prediction_accuracy(&self) -> f64 {
        self.prediction_ema
    }

    /// Total predictions made.
    pub fn prediction_count(&self) -> u64 {
        self.prediction_count
    }

    /// Number of learned causal links.
    pub fn causal_link_count(&self) -> usize {
        self.causal_links.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_bus::DIM_PROLETARIAT;

    #[test]
    fn test_predict_next_state() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);
        let action = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let predicted = model.predict_next_state(&action)?;
        assert_eq!(predicted.dimensions, DIM_PROLETARIAT);

        // Prediction should differ from the original state.
        // HyperMemory::bind uses element-wise multiply on i8 vectors,
        // which produces similarity ~0.5 against the original (not 0.0).
        let sim = predicted.similarity(&model.current_state.representation);
        assert!(sim < 0.9, "Predicted state should differ from current: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_predict_sequence() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);

        let actions: Vec<HyperMemory> = (0..3)
            .map(|_| HyperMemory::generate_seed(DIM_PROLETARIAT))
            .collect();

        let states = model.predict_sequence(&actions)?;
        assert_eq!(states.len(), 3);

        // Each predicted state should differ from the previous.
        for i in 1..states.len() {
            let sim = states[i].similarity(&states[i - 1]);
            assert!(sim < 0.9, "Sequential states should diverge: {:.4}", sim);
        }
        Ok(())
    }

    #[test]
    fn test_record_and_reinforce() {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);

        let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let effect = HyperMemory::generate_seed(DIM_PROLETARIAT);

        model.record_effect(action.clone(), effect.clone());
        assert_eq!(model.causal_link_count(), 1);
        assert_eq!(model.causal_links[0].observations, 1);

        // Record same action again — should reinforce, not create new link.
        model.record_effect(action, effect);
        assert_eq!(model.causal_link_count(), 1);
        assert_eq!(model.causal_links[0].observations, 2);
    }

    #[test]
    fn test_verify_prediction() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);

        let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let predicted = model.predict_next_state(&action)?;

        // Perfect prediction: observed == predicted.
        let verification = model.verify_prediction(&predicted, &predicted, &action);
        assert!(
            (verification.accuracy - 1.0).abs() < 0.01,
            "Perfect prediction should have accuracy ~1.0, got {:.4}", verification.accuracy
        );
        assert!(verification.accurate);

        // Bad prediction: observed is random.
        let random_obs = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let bad = model.verify_prediction(&predicted, &random_obs, &action);
        assert!(bad.accuracy < 0.9, "Random observation should have lower accuracy than perfect");
        Ok(())
    }

    #[test]
    fn test_counterfactual() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state.clone());

        let actual_action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let actual_outcome = state.bind(&actual_action)?;

        let alt_action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let cf = model.counterfactual(&state, &alt_action, &actual_outcome, Some(&goal))?;

        // Counterfactual should produce a different state than actual.
        assert!(cf.divergence < 0.95, "Different action should produce different outcome: {:.4}", cf.divergence);
        // Goal similarity is just a measurement, no assertion on direction.
        assert!(cf.goal_similarity.is_finite());
        Ok(())
    }

    #[test]
    fn test_find_best_action() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);

        // No links → None.
        assert!(model.find_best_action(&HyperMemory::generate_seed(DIM_PROLETARIAT))?.is_none());

        // Record some actions with different effects.
        for _ in 0..5 {
            let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let effect = HyperMemory::generate_seed(DIM_PROLETARIAT);
            model.record_effect(action, effect);
        }

        let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let result = model.find_best_action(&goal)?;
        assert!(result.is_some(), "Should find a best action from 5 links");
        Ok(())
    }

    #[test]
    fn test_update_state_advances_timestamp() {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);
        assert_eq!(model.current_state.timestamp, 0);

        model.update_state(HyperMemory::generate_seed(DIM_PROLETARIAT));
        assert_eq!(model.current_state.timestamp, 1);

        model.update_state(HyperMemory::generate_seed(DIM_PROLETARIAT));
        assert_eq!(model.current_state.timestamp, 2);
    }

    #[test]
    fn test_prediction_accuracy_tracking() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);

        let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let predicted = model.predict_next_state(&action)?;

        // Good prediction.
        model.verify_prediction(&predicted, &predicted, &action);
        assert!(model.prediction_accuracy() > 0.5);

        assert_eq!(model.prediction_count(), 1);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for WorldModel
    // ============================================================

    /// INVARIANT: prediction_accuracy stays in [0.0, 1.0] across any
    /// sequence of verifications.
    #[test]
    fn invariant_prediction_accuracy_in_unit_interval() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);
        for _ in 0..30 {
            let predicted = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let actual = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
            model.verify_prediction(&predicted, &actual, &action);
            let acc = model.prediction_accuracy();
            assert!(acc >= 0.0 && acc <= 1.0,
                "accuracy escaped [0, 1]: {}", acc);
        }
        Ok(())
    }

    /// INVARIANT: prediction_count grows by exactly 1 per verify_prediction.
    #[test]
    fn invariant_prediction_count_monotonic() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);
        for i in 0..15 {
            let before = model.prediction_count();
            let pred = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let actual = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
            model.verify_prediction(&pred, &actual, &action);
            assert_eq!(model.prediction_count(), before + 1,
                "count must grow by 1 at iter {}", i);
        }
        Ok(())
    }

    /// INVARIANT: causal_link_count grows when record_effect is called
    /// with novel action vectors.
    #[test]
    fn invariant_causal_link_count_grows_on_record() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);
        let initial = model.causal_link_count();
        for i in 0..10 {
            let action = HyperMemory::from_string(&format!("action_{}", i), DIM_PROLETARIAT);
            let effect = HyperMemory::from_string(&format!("effect_{}", i), DIM_PROLETARIAT);
            model.record_effect(action, effect);
        }
        assert!(model.causal_link_count() > initial,
            "causal_link_count must grow: {} → {}",
            initial, model.causal_link_count());
        Ok(())
    }

    /// INVARIANT: predict_sequence with N actions returns N predicted states.
    #[test]
    fn invariant_predict_sequence_returns_n_states() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);
        let actions: Vec<HyperMemory> = (0..7)
            .map(|i| HyperMemory::from_string(&format!("a{}", i), DIM_PROLETARIAT))
            .collect();
        let states = model.predict_sequence(&actions)?;
        assert_eq!(states.len(), 7,
            "predict_sequence(7 actions) must return 7 states, got {}", states.len());
        Ok(())
    }

    /// INVARIANT: new() starts with zero predictions and zero causal links.
    #[test]
    fn invariant_new_zero_counters() {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);
        assert_eq!(model.prediction_count(), 0);
        assert_eq!(model.causal_link_count(), 0);
    }

    /// INVARIANT: causal_link_count grows monotonically with distinct record_effect calls.
    #[test]
    fn invariant_causal_link_count_monotone() {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut model = WorldModel::new(state);
        let prev = model.causal_link_count();
        for i in 0..5 {
            let a = HyperMemory::from_string(&format!("action_{}", i), DIM_PROLETARIAT);
            let e = HyperMemory::from_string(&format!("effect_{}", i), DIM_PROLETARIAT);
            model.record_effect(a, e);
            let cur = model.causal_link_count();
            assert!(cur >= prev,
                "causal_link_count should be monotone: {} -> {}", prev, cur);
        }
    }

    /// INVARIANT: prediction_accuracy is in [0,1] on a fresh model (0 before
    /// any predictions run).
    #[test]
    fn invariant_prediction_accuracy_fresh_in_unit_interval() {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);
        let acc = model.prediction_accuracy();
        assert!(acc.is_finite() && (0.0..=1.0).contains(&acc),
            "prediction_accuracy out of [0,1]: {}", acc);
    }

    /// INVARIANT: predict_next_state is pure — same action produces same prediction.
    #[test]
    fn invariant_predict_pure() -> Result<(), Box<dyn std::error::Error>> {
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let model = WorldModel::new(state);
        let action = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let p1 = model.predict_next_state(&action)?;
        let p2 = model.predict_next_state(&action)?;
        let sim = p1.similarity(&p2);
        assert!((sim - 1.0).abs() < 0.001,
            "predict_next_state not pure: sim={}", sim);
        Ok(())
    }
}
