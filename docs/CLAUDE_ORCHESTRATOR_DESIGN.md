# Claude Fleet Orchestrator

## The Problem

Current IPC is file-based — Claude 0 drops markdown files that Claude 1 and 2 may or may not read. No confirmation, no task tracking, no visibility. The user has to manually tell each instance to check for work. This doesn't scale.

## Solution: Orchestrator Service + Fleet Dashboard

### Architecture

```
┌─────────────────────────────────────────────┐
│  Fleet Dashboard (web UI tab in PlausiDen)  │
│  • See all instances, status, current task  │
│  • Assign work from UI                      │
│  • View task queue, completions, failures   │
└──────────────────┬──────────────────────────┘
                   │ REST API
┌──────────────────┴──────────────────────────┐
│  Orchestrator Service (Rust, port 3001)     │
│  • Task queue (SQLite-backed)               │
│  • Instance registry (heartbeat-based)      │
│  • Task assignment (round-robin or manual)  │
│  • Progress tracking                        │
│  • WebSocket push to dashboard              │
└──┬───────────┬───────────┬──────────────────┘
   │           │           │
┌──┴──┐    ┌──┴──┐    ┌──┴──┐
│ C-0 │    │ C-1 │    │ C-2 │    ... C-N
└─────┘    └─────┘    └─────┘
  Each instance polls /api/orchestrator/next-task
  and posts /api/orchestrator/heartbeat + /api/orchestrator/complete
```

### How Instances Connect

Each Claude instance runs a lightweight polling loop:
```bash
# In each Claude's /loop or cron:
TASK=$(curl -s http://localhost:3001/api/tasks/next?instance=claude-1)
# Execute task...
curl -s -X POST http://localhost:3001/api/tasks/complete \
  -d '{"task_id":"...","result":"...","status":"done"}'
```

Or via the existing IPC bus — the orchestrator READS bus.jsonl and WRITES to it, acting as the central coordinator instead of Claude 0 doing it manually.

### Database Schema (orchestrator.db)

```sql
CREATE TABLE instances (
    id TEXT PRIMARY KEY,           -- 'claude-0', 'claude-1', 'claude-2'
    status TEXT DEFAULT 'unknown', -- online, working, idle, offline
    current_task_id TEXT,
    last_heartbeat TEXT,
    pid INTEGER,
    cpu_percent REAL,
    tasks_completed INTEGER DEFAULT 0,
    started_at TEXT
);

CREATE TABLE task_queue (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    priority INTEGER DEFAULT 5,    -- 1=highest, 10=lowest
    status TEXT DEFAULT 'pending',  -- pending, assigned, running, completed, failed
    assigned_to TEXT,               -- instance id
    created_at TEXT DEFAULT (datetime('now')),
    assigned_at TEXT,
    started_at TEXT,
    completed_at TEXT,
    duration_seconds INTEGER,
    result TEXT,
    created_by TEXT DEFAULT 'user', -- user, claude-0, auto
    tags TEXT                       -- comma-separated: 'training,urgent,frontend'
);

CREATE TABLE task_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT,
    instance_id TEXT,
    timestamp TEXT DEFAULT (datetime('now')),
    event TEXT,                     -- assigned, started, progress, completed, failed, heartbeat
    details TEXT
);
```

### API Endpoints

Instance management:
- POST /api/orchestrator/register — instance announces itself
- POST /api/orchestrator/heartbeat — instance reports alive + status
- GET /api/orchestrator/instances — list all instances with status

Task management:
- POST /api/orchestrator/tasks — create a new task
- GET /api/orchestrator/tasks — list all tasks (filterable by status)
- GET /api/orchestrator/tasks/next?instance=X — get next task for instance
- POST /api/orchestrator/tasks/:id/start — mark task as started
- POST /api/orchestrator/tasks/:id/progress — update progress
- POST /api/orchestrator/tasks/:id/complete — mark done with result
- POST /api/orchestrator/tasks/:id/fail — mark failed with reason
- DELETE /api/orchestrator/tasks/:id — cancel a task

Dashboard:
- GET /api/orchestrator/dashboard — everything for the fleet UI
- WS /ws/orchestrator — real-time updates push

### Fleet Dashboard UI (in PlausiDen app)

New top-level section: **Fleet** (or integrate into Admin)

#### Instance Cards
For each Claude instance:
```
┌─────────────────────────────────────────┐
│ Claude 1 (The Refiner)           🟢 WORKING │
│ Current: Generating economics training  │
│ Progress: 45/120 topics                 │
│ Uptime: 3h 22m | Tasks done: 14        │
│ CPU: 23% | Last heartbeat: 4s ago       │
│ [Assign Task] [View Log] [Pause]        │
└─────────────────────────────────────────┘
```

#### Task Queue
Sortable table:
| Priority | Task | Assigned To | Status | Created | Duration |
|----------|------|-------------|--------|---------|----------|
| 🔴 1 | Fix admin UI | claude-2 | Running | 10m ago | 8m |
| 🟡 3 | Generate econ data | claude-1 | Running | 25m ago | 22m |
| ⚪ 5 | Parse Wikidata | unassigned | Pending | 2h ago | — |

#### Create Task (from UI)
Form with: title, description, priority, assign-to (dropdown or auto), tags.
User types what they want done, assigns to a Claude, it shows up in that instance's queue.

#### Timeline
Chronological feed of all events:
- 05:10 Claude-2 completed "AdminModal.tsx" (668 lines)
- 05:08 Claude-1 generated 45 economics training pairs
- 05:05 Claude-0 deployed server with admin dashboard endpoint
- 05:02 User feedback: "admin panel is terrible"

### Why This Solves the Problem

1. **No more file-dropping** — tasks go into a queue, instances poll for work
2. **Visibility** — user sees what each instance is doing in real time
3. **No idle instances** — orchestrator auto-assigns when instance reports idle
4. **Priority management** — urgent tasks get assigned first
5. **History** — full log of what was done, when, by whom
6. **User control** — assign tasks from the UI, not by telling Claude 0 to tell Claude 1
7. **Scalable** — add Claude 3, 4, 5... they just register and start polling
