# AI Visual Presence — The Second Cursor

## Concept

When an AI agent is actively doing something on the user's system — moving files, running commands, interacting with the GUI — the user should SEE it happening. Not in a log buried in a terminal. On screen, with its own visual identity.

## The Second Cursor

A distinct colored cursor (separate from the user's white/default cursor) that appears when any PlausiDen-managed AI is interacting with the desktop.

### Cursor Colors by Agent
- **PlausiDen LFI** — Blue cursor (#3b82f6)
- **Claude Code** — Purple cursor (#8b5cf6)
- **Gemini** — Green cursor (#22c55e)
- **GPT/OpenAI** — Teal cursor (#14b8a6)
- **Ollama (local)** — Orange cursor (#f59e0b)
- **Custom agents** — Assigned from a palette on registration

### Visual Elements

1. **Colored cursor** — moves independently from user's cursor when AI is doing GUI operations (clicking, typing, scrolling)

2. **Action trail** — brief fade trail behind the AI cursor showing its path (like mouse heatmap but live). Fades after 500ms.

3. **Touch indicators** — on mobile/touch screens, show colored circles where the AI "taps" with the agent's color. Pulse animation on tap.

4. **Activity popup** — floating toast/badge near the system tray:
   ```
   ┌─────────────────────────────┐
   │ 🔵 LFI: Analyzing log files │
   │ 🟣 Claude: Editing api.rs   │
   │ 🟢 Gemini: Running tests    │
   └─────────────────────────────┘
   ```
   Shows what each active AI is currently doing. Auto-hides when idle. Click to expand details.

5. **Screen region highlight** — when an AI is working in a specific window or area, a subtle colored border appears around that region. User immediately knows "Claude is working in this terminal."

6. **Command overlay** — when an AI runs a shell command, show a brief overlay:
   ```
   ┌──────────────────────────────────┐
   │ 🟣 Claude executing:             │
   │ cargo build --release            │
   │ [Cancel] [Details]               │
   └──────────────────────────────────┘
   ```

## Implementation Approaches

### Approach 1: X11/Wayland Compositor Extension (native, best)
- Custom compositor layer (wlroots plugin for Wayland, X11 overlay window)
- Draws the second cursor as an overlay above all windows
- Requires: Rust + wayland-client / x11rb crates
- Works system-wide, not just in PlausiDen app

### Approach 2: Desktop Widget (cross-platform, good)
- Transparent always-on-top window
- Renders cursors, trails, activity popups
- Uses GTK4 or Qt6 with transparency
- Works on X11 + Wayland + Windows + macOS

### Approach 3: In-App Only (simplest, limited)
- Only shows within PlausiDen's own windows
- React component for the web dashboard
- Tauri window overlay for desktop app
- Good starting point, expand later

### Recommended: Start with Approach 3, graduate to Approach 2

## In-App Implementation (Phase 1)

Add to the PlausiDen dashboard:

### Activity Bar (always visible)
Bottom of the screen or floating corner widget:
```typescript
interface AgentActivity {
  id: string;
  name: string;
  color: string;
  status: 'working' | 'idle' | 'waiting';
  currentTask: string;
  lastActive: number; // timestamp
}

// Polls /api/orchestrator/instances every 2 seconds
// Shows colored dots + task description for each active agent
```

### Command Notification
When any AI runs a command that affects the system:
```typescript
// Toast notification:
showToast({
  icon: '🟣',
  title: 'Claude Code',
  message: 'Running: cargo build --release',
  actions: ['Cancel', 'View Output'],
  duration: 5000, // auto-dismiss after 5s
  color: '#8b5cf6',
});
```

### File Change Indicator
When an AI modifies a file, show in the activity bar:
```
🟣 Claude modified: lfi_dashboard/src/App.tsx (+386 lines)
```

## Desktop Widget (Phase 2)

A separate Rust binary `plausiden-presence` that:
1. Connects to the orchestrator WebSocket
2. Creates a transparent overlay window
3. Renders colored cursors for active AI agents
4. Shows activity popups near the system tray
5. Draws subtle borders around windows AI agents are working in
6. Captures and displays AI shell commands in real-time

### Technology
- GTK4 + Cairo for rendering (X11 + Wayland compatible)
- D-Bus for desktop notifications
- inotify for file change detection
- ptrace or /proc monitoring for shell command detection

## Mobile (Phase 3)

On Android:
- Floating overlay (like Facebook Chat Heads) showing active AI status
- Colored touch indicators when AI performs actions
- Notification shade integration for ongoing tasks
- Requires SYSTEM_ALERT_WINDOW permission

## Why This Matters

Without visual presence, the user has no idea if the AI is:
- Working or idle
- Making progress or stuck
- About to do something destructive
- Done with a task

The second cursor turns AI from "invisible background process" into "visible collaborator sitting next to you." This is the difference between "I hope it's working" and "I can see exactly what it's doing."

No other AI platform has this. It's a genuine UX innovation.
