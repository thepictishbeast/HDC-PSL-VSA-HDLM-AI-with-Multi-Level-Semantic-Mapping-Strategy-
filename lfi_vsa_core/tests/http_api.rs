//! HTTP integration tests for the axum API.
//!
//! These exercise the real router with real handlers — catching wiring
//! bugs (wrong method, wrong path, missing state) that unit tests on
//! handlers would miss.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// GET /api/health must return 200 + an `ok` field.
#[tokio::test]
async fn test_health_endpoint_returns_ok_flag() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert!(json.get("ok").is_some(), "must have `ok` field, got {}", json);
    assert!(json.get("subsystems").is_some(), "must have `subsystems` field");
}

/// GET /api/provenance/stats must return a trace_count (0 initially).
#[tokio::test]
async fn test_provenance_stats_empty_initially() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/provenance/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["trace_count"].as_u64().expect("trace_count"), 0);
    assert_eq!(json["has_traces"].as_bool().expect("has_traces"), false);
}

/// POST /api/think returns a conclusion_id + answer on valid input.
#[tokio::test]
async fn test_think_endpoint_returns_conclusion_id() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({ "input": "what is sovereignty" }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/think")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "ok");
    assert!(json.get("conclusion_id").is_some(), "must have conclusion_id");
    assert!(json.get("answer").is_some(), "must have answer");
}

/// POST /api/think rejects inputs larger than 16 KiB.
#[tokio::test]
async fn test_think_endpoint_rejects_oversize() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    // 16 KiB + 1
    let huge = "a".repeat(16 * 1024 + 1);
    let body = serde_json::json!({ "input": huge }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/think")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected");
}

/// GET /api/provenance/:cid returns ReconstructedRationalization for an unknown id.
#[tokio::test]
async fn test_provenance_unknown_cid_is_reconstructed() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/provenance/99999999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let kind = json["kind"]["kind"].as_str().expect("kind");
    assert_eq!(kind, "ReconstructedRationalization",
        "unknown cid must return Reconstructed, got {}", kind);
}

/// POST /api/provenance/reset requires authentication — unauthenticated rejected.
#[tokio::test]
async fn test_provenance_reset_requires_auth() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/provenance/reset")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected",
        "unauthenticated reset must be rejected, got {}", json);
}

/// GET /api/knowledge/due returns an empty list on a fresh agent when
/// no concepts have been registered with the scheduler beyond the core
/// seed set — but it must still return 200 with a `count` field.
#[tokio::test]
async fn test_knowledge_due_endpoint_returns_count() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/knowledge/due")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert!(json.get("count").is_some(), "must have count field");
    assert!(json["concepts"].is_array(), "concepts must be array");
}

/// POST /api/knowledge/review rejects empty concept strings.
#[tokio::test]
async fn test_knowledge_review_rejects_empty_concept() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({ "concept": "", "quality": 5 }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/knowledge/review")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected");
}

/// GET /api/metrics returns 200 with Prometheus text content type.
#[tokio::test]
async fn test_metrics_endpoint_returns_200_and_text_plain() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(Request::builder().uri("/api/metrics").body(Body::empty()).unwrap())
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.starts_with("text/plain"),
        "metrics must be text/plain, got {:?}", content_type);
    // Body parses as UTF-8 (may be empty if no counter has been incremented yet).
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let _text = std::str::from_utf8(&bytes).expect("utf8");
}

/// GET /api/agent/state returns the consolidated dashboard view.
#[tokio::test]
async fn test_agent_state_returns_consolidated_summary() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/agent/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");

    // All four sections must be present.
    assert!(json.get("psl").is_some(), "missing psl section");
    assert!(json.get("knowledge").is_some(), "missing knowledge section");
    assert!(json.get("provenance").is_some(), "missing provenance section");
    assert!(json.get("agent").is_some(), "missing agent section");

    // PSL has at least the 5 default axioms.
    assert!(json["psl"]["axiom_count"].as_u64().unwrap() >= 5);
    // Knowledge has the seeded concepts.
    assert!(json["knowledge"]["concept_count"].as_u64().unwrap() > 5);
    // Provenance starts empty.
    assert_eq!(json["provenance"]["trace_count"].as_u64().unwrap(), 0);
    // Agent starts unauthenticated.
    assert_eq!(json["agent"]["authenticated"].as_bool().unwrap(), false);
}

/// POST /api/opsec/scan sanitizes input and reports matches.
#[tokio::test]
async fn test_opsec_scan_redacts_sensitive_text() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({
        "text": "Contact me at user@example.com or 555-12-3456 about the test."
    }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/opsec/scan")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "ok");
    let sanitized = json["sanitized"].as_str().expect("sanitized");
    // The original PII (email + SSN) must NOT appear in the sanitized text.
    assert!(!sanitized.contains("user@example.com"),
        "email must be redacted, got: {}", sanitized);
    assert!(!sanitized.contains("555-12-3456"),
        "SSN must be redacted, got: {}", sanitized);
}

/// POST /api/opsec/scan rejects empty + oversize inputs.
#[tokio::test]
async fn test_opsec_scan_rejects_empty_and_oversize() {
    let app = lfi_vsa_core::api::create_router().expect("router");

    // Empty
    let body = serde_json::json!({ "text": "" }).to_string();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/opsec/scan")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected", "empty must be rejected");

    // Oversize
    let huge = "a".repeat(64 * 1024 + 1);
    let body = serde_json::json!({ "text": huge }).to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/opsec/scan")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected", "oversize must be rejected");
}

/// POST /api/audit returns a verdict for any non-empty seed.
#[tokio::test]
async fn test_audit_endpoint_returns_verdict() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({ "seed": "audit me" }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/audit")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "ok");
    assert!(json.get("axiom_id").is_some(), "must have axiom_id");
    assert!(json.get("confidence").is_some(), "must have confidence");
    assert!(json.get("permits_execution").is_some(), "must have permits_execution");
}

/// POST /api/audit rejects empty seeds.
#[tokio::test]
async fn test_audit_endpoint_rejects_empty_seed() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({ "seed": "" }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/audit")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected");
}

/// POST /api/knowledge/learn requires authentication.
#[tokio::test]
async fn test_knowledge_learn_requires_auth() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let body = serde_json::json!({
        "concept": "test_concept_via_http",
        "related": ["a", "b"]
    }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/knowledge/learn")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected",
        "unauthenticated learn must be rejected, got {}", json);
}

/// POST /api/knowledge/learn rejects oversize related list.
#[tokio::test]
async fn test_knowledge_learn_rejects_oversize_related() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let mut related = Vec::with_capacity(100);
    for i in 0..100 { related.push(format!("rel_{}", i)); }
    let body = serde_json::json!({
        "concept": "x",
        "related": related,
    }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/knowledge/learn")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(json["status"], "rejected");
}

/// GET /api/knowledge/concepts returns a list with mastery + relations.
#[tokio::test]
async fn test_knowledge_concepts_endpoint_lists_seeded_concepts() {
    let app = lfi_vsa_core::api::create_router().expect("router");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/knowledge/concepts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("valid JSON");
    let concepts = json["concepts"].as_array().expect("concepts is array");
    // KnowledgeEngine seeds core concepts on init — there must be > 5.
    assert!(concepts.len() > 5,
        "must have seeded concepts (got {})", concepts.len());
    // Each entry must have name, mastery, encounter_count.
    let first = &concepts[0];
    assert!(first.get("name").is_some());
    assert!(first.get("mastery").is_some());
    assert!(first.get("encounter_count").is_some());
}

/// /api/think → /api/metrics should show the counter has incremented.
#[tokio::test]
async fn test_think_increments_think_counter() {
    let app = lfi_vsa_core::api::create_router().expect("router");

    // Hit /api/think a few times.
    for input in ["q1", "q2", "q3"] {
        let body = serde_json::json!({ "input": input }).to_string();
        let _ = app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/think")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .expect("think req");
    }

    // Now scrape metrics.
    let response = app
        .oneshot(Request::builder().uri("/api/metrics").body(Body::empty()).unwrap())
        .await
        .expect("metrics req");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).expect("utf8");
    // The counter line must show ≥ 3 (we made 3 calls — exact match would
    // be brittle if other tests interleave, so just assert presence + non-zero).
    assert!(text.contains("lfi_think_total"),
        "lfi_think_total must be exposed");
    let any_nonzero = text.lines().any(|l| {
        l.starts_with("lfi_think_total") && l.split_whitespace().last()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|n| n >= 3)
            .unwrap_or(false)
    });
    assert!(any_nonzero, "lfi_think_total must be ≥ 3 after 3 think calls, got: {}", text);
}

/// End-to-end: POST /api/think, then GET /api/provenance/:cid returns Traced.
#[tokio::test]
async fn test_think_then_query_provenance_returns_traced() {
    let app = lfi_vsa_core::api::create_router().expect("router");

    // Fire a think call.
    let think_body = serde_json::json!({ "input": "end-to-end trace round-trip" }).to_string();
    let think_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/think")
                .header("content-type", "application/json")
                .body(Body::from(think_body))
                .unwrap(),
        )
        .await
        .expect("think req");
    assert_eq!(think_resp.status(), StatusCode::OK);
    let think_bytes = think_resp.into_body().collect().await.unwrap().to_bytes();
    let think_json: Value = serde_json::from_slice(&think_bytes).expect("json");
    let cid = think_json["conclusion_id"].as_u64().expect("cid");

    // Now query that cid back.
    let prov_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/provenance/{}", cid))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("prov req");
    assert_eq!(prov_resp.status(), StatusCode::OK);
    let prov_bytes = prov_resp.into_body().collect().await.unwrap().to_bytes();
    let prov_json: Value = serde_json::from_slice(&prov_bytes).expect("json");
    let kind = prov_json["kind"]["kind"].as_str().expect("kind");
    assert_eq!(kind, "TracedDerivation",
        "cid from a /api/think call must be TracedDerivation end-to-end, got {:?}", prov_json);
}
