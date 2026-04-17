//! # PlausiDen Orchestrator — Claude Fleet Management
//!
//! Central service for managing multiple Claude Code instances.
//! Provides: task queue, instance registry, heartbeat monitoring,
//! progress tracking, and a REST API for the fleet dashboard.
//!
//! Runs on port 3001 alongside the main server on 3000.

use axum::{extract::State, routing::{get, post, delete}, Router, Json};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

struct AppState {
    db: Mutex<Connection>,
}

// ============================================================
// Models
// ============================================================

#[derive(Serialize, Deserialize, Debug)]
struct Instance {
    id: String,
    status: String,
    current_task_id: Option<String>,
    last_heartbeat: Option<String>,
    tasks_completed: i64,
    pid: Option<i64>,
    cpu_percent: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    id: String,
    title: String,
    description: Option<String>,
    priority: i32,
    status: String,
    assigned_to: Option<String>,
    created_at: String,
    assigned_at: Option<String>,
    completed_at: Option<String>,
    duration_seconds: Option<i64>,
    result: Option<String>,
    created_by: String,
    tags: Option<String>,
}

#[derive(Deserialize)]
struct CreateTask {
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    assign_to: Option<String>,
    tags: Option<String>,
}

#[derive(Deserialize)]
struct Heartbeat {
    instance_id: String,
    status: Option<String>,
    pid: Option<i64>,
    cpu_percent: Option<f64>,
    current_task: Option<String>,
}

#[derive(Deserialize)]
struct CompleteTask {
    result: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct NextTaskQuery {
    instance: String,
}

// ============================================================
// Database
// ============================================================

fn init_db(conn: &Connection) {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS instances (
            id TEXT PRIMARY KEY,
            status TEXT DEFAULT 'unknown',
            current_task_id TEXT,
            last_heartbeat TEXT,
            pid INTEGER,
            cpu_percent REAL,
            tasks_completed INTEGER DEFAULT 0,
            started_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS task_queue (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            priority INTEGER DEFAULT 5,
            status TEXT DEFAULT 'pending',
            assigned_to TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            assigned_at TEXT,
            started_at TEXT,
            completed_at TEXT,
            duration_seconds INTEGER,
            result TEXT,
            created_by TEXT DEFAULT 'user',
            tags TEXT
        );
        CREATE TABLE IF NOT EXISTS task_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT,
            instance_id TEXT,
            timestamp TEXT DEFAULT (datetime('now')),
            event TEXT,
            details TEXT
        );
    ").expect("Failed to init orchestrator DB");
}

// ============================================================
// Handlers
// ============================================================

async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());

    let instances: Vec<serde_json::Value> = {
        let mut stmt = db.prepare("SELECT id, status, current_task_id, last_heartbeat, tasks_completed, pid, cpu_percent FROM instances ORDER BY id").unwrap();
        stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0).unwrap_or_default(),
                "status": row.get::<_,String>(1).unwrap_or_default(),
                "current_task_id": row.get::<_,Option<String>>(2).unwrap_or(None),
                "last_heartbeat": row.get::<_,Option<String>>(3).unwrap_or(None),
                "tasks_completed": row.get::<_,i64>(4).unwrap_or(0),
                "pid": row.get::<_,Option<i64>>(5).unwrap_or(None),
                "cpu_percent": row.get::<_,Option<f64>>(6).unwrap_or(None),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };

    let tasks: Vec<serde_json::Value> = {
        let mut stmt = db.prepare("SELECT id, title, priority, status, assigned_to, created_at, duration_seconds, tags FROM task_queue ORDER BY priority ASC, created_at DESC LIMIT 50").unwrap();
        stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0).unwrap_or_default(),
                "title": row.get::<_,String>(1).unwrap_or_default(),
                "priority": row.get::<_,i32>(2).unwrap_or(5),
                "status": row.get::<_,String>(3).unwrap_or_default(),
                "assigned_to": row.get::<_,Option<String>>(4).unwrap_or(None),
                "created_at": row.get::<_,String>(5).unwrap_or_default(),
                "duration_seconds": row.get::<_,Option<i64>>(6).unwrap_or(None),
                "tags": row.get::<_,Option<String>>(7).unwrap_or(None),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };

    let recent_log: Vec<serde_json::Value> = {
        let mut stmt = db.prepare("SELECT task_id, instance_id, timestamp, event, details FROM task_log ORDER BY id DESC LIMIT 30").unwrap();
        stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "task_id": row.get::<_,Option<String>>(0).unwrap_or(None),
                "instance_id": row.get::<_,Option<String>>(1).unwrap_or(None),
                "timestamp": row.get::<_,String>(2).unwrap_or_default(),
                "event": row.get::<_,String>(3).unwrap_or_default(),
                "details": row.get::<_,Option<String>>(4).unwrap_or(None),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };

    let pending: i64 = db.query_row("SELECT count(*) FROM task_queue WHERE status='pending'", [], |r| r.get(0)).unwrap_or(0);
    let running: i64 = db.query_row("SELECT count(*) FROM task_queue WHERE status='running'", [], |r| r.get(0)).unwrap_or(0);
    let completed: i64 = db.query_row("SELECT count(*) FROM task_queue WHERE status='completed'", [], |r| r.get(0)).unwrap_or(0);

    Json(serde_json::json!({
        "instances": instances,
        "tasks": tasks,
        "log": recent_log,
        "summary": { "pending": pending, "running": running, "completed": completed },
    }))
}

async fn register_handler(State(state): State<Arc<AppState>>, Json(hb): Json<Heartbeat>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    db.execute(
        "INSERT OR REPLACE INTO instances (id, status, last_heartbeat, pid, cpu_percent) VALUES (?1, ?2, datetime('now'), ?3, ?4)",
        params![hb.instance_id, hb.status.unwrap_or("online".into()), hb.pid, hb.cpu_percent],
    ).ok();
    Json(serde_json::json!({"status": "registered", "instance": hb.instance_id}))
}

async fn heartbeat_handler(State(state): State<Arc<AppState>>, Json(hb): Json<Heartbeat>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    db.execute(
        "UPDATE instances SET status=?2, last_heartbeat=datetime('now'), pid=?3, cpu_percent=?4, current_task_id=?5 WHERE id=?1",
        params![hb.instance_id, hb.status.unwrap_or("online".into()), hb.pid, hb.cpu_percent, hb.current_task],
    ).ok();
    Json(serde_json::json!({"status": "ok"}))
}

async fn list_instances(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let instances: Vec<serde_json::Value> = {
        let mut stmt = db.prepare("SELECT id, status, current_task_id, last_heartbeat, tasks_completed FROM instances ORDER BY id").unwrap();
        stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0).unwrap_or_default(),
                "status": row.get::<_,String>(1).unwrap_or_default(),
                "current_task_id": row.get::<_,Option<String>>(2).unwrap_or(None),
                "last_heartbeat": row.get::<_,Option<String>>(3).unwrap_or(None),
                "tasks_completed": row.get::<_,i64>(4).unwrap_or(0),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };
    Json(serde_json::json!({"instances": instances}))
}

async fn create_task_handler(State(state): State<Arc<AppState>>, Json(req): Json<CreateTask>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let id = uuid::Uuid::new_v4().to_string();
    let priority = req.priority.unwrap_or(5);
    let status = if req.assign_to.is_some() { "assigned" } else { "pending" };

    db.execute(
        "INSERT INTO task_queue (id, title, description, priority, status, assigned_to, created_by, tags, assigned_at) VALUES (?1,?2,?3,?4,?5,?6,'user',?7, CASE WHEN ?6 IS NOT NULL THEN datetime('now') ELSE NULL END)",
        params![id, req.title, req.description, priority, status, req.assign_to, req.tags],
    ).ok();

    // Log it
    db.execute("INSERT INTO task_log (task_id, event, details) VALUES (?1, 'created', ?2)",
        params![id, req.title]).ok();

    if let Some(ref assignee) = req.assign_to {
        db.execute("INSERT INTO task_log (task_id, instance_id, event, details) VALUES (?1, ?2, 'assigned', ?3)",
            params![id, assignee, req.title]).ok();
        db.execute("UPDATE instances SET current_task_id=?1, status='working' WHERE id=?2",
            params![id, assignee]).ok();
    }

    Json(serde_json::json!({"id": id, "status": status, "assigned_to": req.assign_to}))
}

async fn list_tasks(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let tasks: Vec<serde_json::Value> = {
        let mut stmt = db.prepare("SELECT id, title, priority, status, assigned_to, created_at, tags FROM task_queue ORDER BY priority ASC, created_at DESC LIMIT 100").unwrap();
        stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0).unwrap_or_default(),
                "title": row.get::<_,String>(1).unwrap_or_default(),
                "priority": row.get::<_,i32>(2).unwrap_or(5),
                "status": row.get::<_,String>(3).unwrap_or_default(),
                "assigned_to": row.get::<_,Option<String>>(4).unwrap_or(None),
                "created_at": row.get::<_,String>(5).unwrap_or_default(),
                "tags": row.get::<_,Option<String>>(6).unwrap_or(None),
            }))
        }).unwrap().filter_map(|r| r.ok()).collect()
    };
    Json(serde_json::json!({"tasks": tasks}))
}

async fn next_task_handler(State(state): State<Arc<AppState>>, axum::extract::Query(q): axum::extract::Query<NextTaskQuery>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());

    // Find highest-priority pending or assigned-to-this-instance task
    let task: Option<(String, String, Option<String>)> = db.query_row(
        "SELECT id, title, description FROM task_queue WHERE (status='pending' OR (status='assigned' AND assigned_to=?1)) ORDER BY priority ASC, created_at ASC LIMIT 1",
        params![q.instance],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).ok();

    match task {
        Some((id, title, desc)) => {
            db.execute("UPDATE task_queue SET status='running', assigned_to=?2, started_at=datetime('now') WHERE id=?1",
                params![id, q.instance]).ok();
            db.execute("UPDATE instances SET current_task_id=?1, status='working' WHERE id=?2",
                params![id, q.instance]).ok();
            db.execute("INSERT INTO task_log (task_id, instance_id, event) VALUES (?1, ?2, 'started')",
                params![id, q.instance]).ok();
            Json(serde_json::json!({"task_id": id, "title": title, "description": desc}))
        }
        None => Json(serde_json::json!({"task_id": null, "message": "no tasks available"})),
    }
}

async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(body): Json<CompleteTask>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let status = body.status.unwrap_or("completed".into());

    db.execute(
        "UPDATE task_queue SET status=?2, completed_at=datetime('now'), result=?3, duration_seconds=CAST((julianday('now')-julianday(started_at))*86400 AS INTEGER) WHERE id=?1",
        params![task_id, status, body.result],
    ).ok();

    // Update instance
    let instance: Option<String> = db.query_row("SELECT assigned_to FROM task_queue WHERE id=?1", params![task_id], |r| r.get(0)).ok();
    if let Some(ref inst) = instance {
        db.execute("UPDATE instances SET current_task_id=NULL, status='idle', tasks_completed=tasks_completed+1 WHERE id=?1", params![inst]).ok();
    }

    db.execute("INSERT INTO task_log (task_id, instance_id, event, details) VALUES (?1, ?2, ?3, ?4)",
        params![task_id, instance, status, body.result]).ok();

    Json(serde_json::json!({"status": status, "task_id": task_id}))
}

async fn delete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    db.execute("DELETE FROM task_queue WHERE id=?1", params![task_id]).ok();
    db.execute("INSERT INTO task_log (task_id, event) VALUES (?1, 'deleted')", params![task_id]).ok();
    Json(serde_json::json!({"status": "deleted", "task_id": task_id}))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_path = std::env::var("ORCHESTRATOR_DB")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or("/root".into());
            format!("{}/.local/share/plausiden/orchestrator.db", home)
        });

    let conn = Connection::open(&db_path).expect("Failed to open orchestrator DB");
    init_db(&conn);

    let state = Arc::new(AppState { db: Mutex::new(conn) });

    let app = Router::new()
        .route("/api/orchestrator/dashboard", get(dashboard_handler))
        .route("/api/orchestrator/register", post(register_handler))
        .route("/api/orchestrator/heartbeat", post(heartbeat_handler))
        .route("/api/orchestrator/instances", get(list_instances))
        .route("/api/orchestrator/tasks", get(list_tasks).post(create_task_handler))
        .route("/api/orchestrator/tasks/next", get(next_task_handler))
        .route("/api/orchestrator/tasks/:id/complete", post(complete_task_handler))
        .route("/api/orchestrator/tasks/:id", delete(delete_task_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("Orchestrator listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server failed");
}
