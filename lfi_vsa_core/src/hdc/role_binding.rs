// ============================================================
// HDC Role-Filler Binding — VSA Substrate II encoder
//
// Per LFI_SUPERSOCIETY_ARCHITECTURE.md §Substrate-II:
//     fact_hv = R_subject  ⊗ E_entity
//             + R_predicate ⊗ E_relation
//             + R_object    ⊗ E_value
//
// where ⊗ is bipolar binding (XOR) and + is bundling (majority vote).
// Unbinding with any role approximately recovers that role's filler:
//     unbind(fact_hv, R_subject) ≈ E_entity       (up to bundle interference)
//
// This is the encoder every (subj, pred, obj) tuple ingestion goes
// through. Output is a single 10,000-dim BipolarVector (1.25 KB) that
// participates in cosine similarity, resonator factorization, causal
// DAG edges, and prototype bundling.
//
// SUPERSOCIETY: post-LLM fact representation. No tokens, no attention,
// no next-token prediction. Deterministic, compositional, algebraic.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Canonical namespaces for seed derivation.
///
/// Role vectors and concept vectors both derive from 64-bit seeds via
/// BipolarVector::from_seed. Namespacing prevents collision between the
/// role `subject` and a concept someone happened to name "subject".
const NS_ROLE: &str = "LFI::ROLE::";
const NS_CONCEPT: &str = "LFI::CONCEPT::";

/// Deterministic 64-bit hash (FNV-style via DefaultHasher). Not a
/// cryptographic hash — sufficient for keying on-the-fly codebook
/// generation. Same name across process restarts always maps to the
/// same hypervector, which is the property we rely on.
fn stable_hash(namespace: &str, name: &str) -> u64 {
    let mut h = DefaultHasher::new();
    namespace.hash(&mut h);
    name.hash(&mut h);
    h.finish()
}

/// Canonical role hypervector for `name` (e.g. "subject", "predicate",
/// "object", or any custom role like "cause" / "effect" / "speaker").
///
/// Deterministic: same role name → same vector across the entire fleet.
/// Quasi-orthogonal to other role names (in 10k-dim space, two random
/// bipolar vectors have expected cosine ~0 with std-dev 1/√D).
pub fn role_vector(name: &str) -> BipolarVector {
    BipolarVector::from_seed(stable_hash(NS_ROLE, name))
}

/// Canonical concept (filler) hypervector for `name`.
///
/// Any string concept — entity name, predicate label, value token —
/// maps to a single deterministic hypervector. Two different concepts
/// map to quasi-orthogonal vectors with high probability.
///
/// SUPERSOCIETY: language-agnostic at the cognitive layer. "water",
/// "agua", "水" each produce a different concept vector, but the
/// HDLM multilingual codebook (#342, #274) will later bundle them at
/// the same synset so facts bind to one shared concept regardless of
/// surface language.
pub fn concept_vector(name: &str) -> BipolarVector {
    BipolarVector::from_seed(stable_hash(NS_CONCEPT, name))
}

/// Bind a (role, filler) pair into a single hypervector.
///
/// This is `R ⊗ E` in the VSA algebra. Both inputs deterministic in,
/// deterministic out. XOR is self-inverse, so `unbind(bind(R, E), R) = E`.
pub fn bind_role(role: &BipolarVector, filler: &BipolarVector) -> Result<BipolarVector, HdcError> {
    role.bind(filler)
}

/// Encode a `(subject, predicate, object)` tuple as a single fact
/// hypervector.
///
/// Pipeline:
///   R_subj   = role_vector("subject")
///   R_pred   = role_vector("predicate")
///   R_obj    = role_vector("object")
///   E_s      = concept_vector(subject)
///   E_p      = concept_vector(predicate)
///   E_o      = concept_vector(object)
///   fact_hv  = bundle(bind(R_subj,E_s), bind(R_pred,E_p), bind(R_obj,E_o))
///
/// Output: a single 10,000-dim BipolarVector representing the complete
/// fact, ready to bundle into prototypes / insert into the fact store /
/// feed into resonator factorization.
///
/// AVP-PASS-1: Tier 1 — all inputs validated via BipolarVector::bind's
/// internal check_dim. No unwraps, no panics, returns HdcError on any
/// primitive failure.
pub fn encode_tuple(subject: &str, predicate: &str, object: &str) -> Result<BipolarVector, HdcError> {
    let r_subj = role_vector("subject");
    let r_pred = role_vector("predicate");
    let r_obj = role_vector("object");
    let e_s = concept_vector(subject);
    let e_p = concept_vector(predicate);
    let e_o = concept_vector(object);

    let b_s = bind_role(&r_subj, &e_s)?;
    let b_p = bind_role(&r_pred, &e_p)?;
    let b_o = bind_role(&r_obj, &e_o)?;

    BipolarVector::bundle(&[&b_s, &b_p, &b_o])
}

/// Unbind a fact hypervector against a role, returning the noisy filler
/// approximation. In a D=10,000 bipolar fact bundled from 3 role-filler
/// pairs, the noise is modest — cosine against the true filler is
/// typically >0.45, against any other random codebook entry ≈0. Use
/// with a cleanup step (nearest-concept lookup) to recover the exact
/// filler for large codebooks; resonator factorization (task #340)
/// generalizes this for composite bindings.
///
/// Property: `unbind(encode_tuple("water","boils_at","100C"), "predicate")`
/// should be closer (cosine) to `concept_vector("boils_at")` than to
/// any other random concept.
pub fn unbind_role(fact_hv: &BipolarVector, role_name: &str) -> Result<BipolarVector, HdcError> {
    let r = role_vector(role_name);
    r.bind(fact_hv) // XOR is self-inverse
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_vectors_deterministic() {
        let r1 = role_vector("subject");
        let r2 = role_vector("subject");
        // Deterministic: same name twice → identical vector.
        assert_eq!(r1.data, r2.data);
    }

    #[test]
    fn role_vectors_distinct() {
        let r_s = role_vector("subject");
        let r_p = role_vector("predicate");
        let r_o = role_vector("object");
        // Different names → different vectors (with astronomical
        // probability at D=10,000).
        assert_ne!(r_s.data, r_p.data);
        assert_ne!(r_p.data, r_o.data);
        assert_ne!(r_s.data, r_o.data);
    }

    #[test]
    fn concept_vectors_distinct() {
        let v1 = concept_vector("water");
        let v2 = concept_vector("fire");
        assert_ne!(v1.data, v2.data);
    }

    #[test]
    fn role_and_concept_namespaces_dont_collide() {
        // A concept named "subject" must NOT equal the role "subject".
        // This is what the namespace prefix is for.
        let role = role_vector("subject");
        let concept = concept_vector("subject");
        assert_ne!(role.data, concept.data);
    }

    #[test]
    fn encode_tuple_produces_correct_dimensionality() {
        let fact = encode_tuple("water", "boils_at", "100C").unwrap();
        assert_eq!(fact.dim(), 10_000);
    }

    #[test]
    fn encode_tuple_deterministic() {
        let f1 = encode_tuple("water", "boils_at", "100C").unwrap();
        let f2 = encode_tuple("water", "boils_at", "100C").unwrap();
        assert_eq!(f1.data, f2.data);
    }

    #[test]
    fn unbind_recovers_predicate_filler_better_than_noise() {
        // The acceptance test from task #330: encode a real tuple,
        // unbind by R_predicate, confirm the result is closer to the
        // true predicate filler than to unrelated concepts.
        let fact = encode_tuple("water", "boils_at", "100C").unwrap();
        let recovered = unbind_role(&fact, "predicate").unwrap();

        let true_filler = concept_vector("boils_at");
        let distractor_1 = concept_vector("water");
        let distractor_2 = concept_vector("100C");
        let distractor_3 = concept_vector("flies");

        let sim_true = recovered.similarity(&true_filler).unwrap();
        let sim_d1 = recovered.similarity(&distractor_1).unwrap();
        let sim_d2 = recovered.similarity(&distractor_2).unwrap();
        let sim_d3 = recovered.similarity(&distractor_3).unwrap();

        // The true filler must be the closest match. With bundling of
        // 3 role-filler pairs the signal ≈ 0.5 (raw bundle) and
        // distractors ≈ 0, so a generous margin is safe.
        assert!(sim_true > sim_d1, "true {:.3} should exceed subj-distractor {:.3}", sim_true, sim_d1);
        assert!(sim_true > sim_d2, "true {:.3} should exceed obj-distractor {:.3}", sim_true, sim_d2);
        assert!(sim_true > sim_d3, "true {:.3} should exceed random-distractor {:.3}", sim_true, sim_d3);
    }

    #[test]
    fn unbind_recovers_subject_and_object_fillers() {
        let fact = encode_tuple("mammal", "is_a", "animal").unwrap();
        let rs = unbind_role(&fact, "subject").unwrap();
        let ro = unbind_role(&fact, "object").unwrap();

        let true_subj = concept_vector("mammal");
        let true_obj = concept_vector("animal");

        let subj_true = rs.similarity(&true_subj).unwrap();
        let subj_obj = rs.similarity(&true_obj).unwrap();
        let obj_true = ro.similarity(&true_obj).unwrap();
        let obj_subj = ro.similarity(&true_subj).unwrap();

        // Unbinding by R_subject must surface the subject filler over
        // the object filler, and vice versa.
        assert!(subj_true > subj_obj,
            "unbind(subject): subj_true={:.3} should exceed subj_obj={:.3}", subj_true, subj_obj);
        assert!(obj_true > obj_subj,
            "unbind(object): obj_true={:.3} should exceed obj_subj={:.3}", obj_true, obj_subj);
    }

    #[test]
    fn different_tuples_are_distinct() {
        // Two unrelated facts must encode to distinguishable vectors.
        let f1 = encode_tuple("water", "boils_at", "100C").unwrap();
        let f2 = encode_tuple("fire", "is_hot", "yes").unwrap();
        let sim = f1.similarity(&f2).unwrap();
        // Should be near orthogonal (cosine ≈ 0 for random bundles).
        assert!(sim.abs() < 0.2, "unrelated tuples should be near-orthogonal, got {:.3}", sim);
    }
}
