# PlausiDen CLI Audit

This doc captures the invariants the PlausiDen CLI MUST hold to match
Claude Code / Gemini CLI / `aichat` quality. It doubles as a design spec
for the CLI (not yet built) so the first commit lands correctly.

Three layers:

1. **Static lint** (`scripts/cli-lint.sh`) — grep checks on the Rust CLI
   source once it exists.
2. **Runtime checklist** (below) — manual gates against real terminal
   behaviour.
3. **Self-test mode** (`plausiden --self-check`) — TTY + color + piping
   + signal handling smoke suite the CLI runs on demand.

---

## Feel & behaviour parity

PlausiDen CLI should feel like Claude Code / Gemini CLI:

- **Interactive REPL** with multiline editing (shift+enter / \ continuation).
- **Streaming output** — prints `chat_chunks` / `chat_progress` events
  as they arrive, not after `chat_done`.
- **Slash commands** (`/help`, `/clear`, `/new`, `/quit`, `/history`,
  `/fact <key>`, `/resolve <lang> <text>`, `/verify <key>`, `/copy-last`).
- **Cmd palette via `@`** (like Claude's @-mention) to invoke by name.
- **History recall** — up-arrow walks previous inputs;
  readline-compatible shortcuts (Ctrl+A/E/U/K/W/R).
- **Tab completion** — command names + fact_keys from the local cache +
  filenames in `$PWD`.
- **Rich TTY**: 24-bit color when `$COLORTERM` allows, 256-color
  fallback, monochrome fallback when not a TTY.
- **Animated progress**: spinner during backend-wait, progress bar for
  ingest runs.
- **Structured boxes** for important output (fact popover, contradictions
  list) — Unicode box-drawing, respects terminal width.

---

## Invariants

### Input
- **Multiline**: `shift+enter` / `\<return>` or `"""` heredoc continues the input.
- **History**: persisted to `$XDG_DATA_HOME/plausiden/history`. Capped
  at 10k lines. Duplicates consecutive entries are dropped.
- **Readline**: use `rustyline` or `reedline` — standard emacs keybindings.
- **Ctrl+C**: cancels the current turn (interrupts the SSE/WS stream),
  does NOT exit the CLI. A second Ctrl+C within 2s exits.
- **Ctrl+D** on empty line: clean exit with "Bye.".
- **Paste detection** (bracketed paste mode): when ≥ 5 lines pasted
  at once, prompt `(paste? [y/N])` before submitting to avoid
  accidental multi-line submit.

### Output
- **TTY-aware**: colored output ONLY when `stdout.is_terminal()` AND
  `$NO_COLOR` is unset. Otherwise plain ASCII.
- **`--json`** flag: machine-readable output on every command — stable
  schema documented in `docs/CLI_JSON_SCHEMA.md`.
- **Streaming**: `chat_chunk` events print as they arrive via a
  `stdout.flush()` loop. Never buffer a whole response.
- **Fact citations** (`[fact:KEY] (source: X, similarity N%)`) rendered
  as dim-underlined tokens in interactive mode; preserved as literal
  text in `--json` / pipe mode so downstream tools can parse them.
- **Progress bars**: `indicatif` or equivalent — render only when
  `stderr.is_terminal()`, never to a redirected stderr.
- **Tables** via `comfy-table` — column widths derived from
  `term_size::dimensions()`, wrap at `$COLUMNS` or 100.
- **Error output** goes to stderr; exit codes 0 / 1 / 2 per convention
  (0 ok, 1 runtime failure, 2 usage error).

### Commands
- **`plausiden` (no args)**: enters interactive REPL.
- **`plausiden "prompt"`**: one-shot, streams reply, exits on `chat_done`.
- **`plausiden -p "prompt"`** / **`plausiden --print "prompt"`**: same as
  above, always non-interactive (matches Claude Code's `-p` flag).
- **`plausiden /fact <key>`**: prints the fact popover content.
- **`plausiden /verify <key>`**: POST /api/proof/verify, prints verdict.
- **`plausiden /resolve <lang> <text>`**: GET /api/concepts/resolve, prints concept_id.
- **`plausiden /ingest <corpus> [--limit N]`**: POST /api/ingest/start +
  stream progress.
- **`plausiden /drift`**: one-shot /api/drift/snapshot with a rendered table.
- **`plausiden /tokens list|issue|revoke`**: capability-token management.
- **`plausiden --self-check`**: runs the CLI test suite (see below) and
  prints PASS / FAIL.

### Configuration
- **Config** in `$XDG_CONFIG_HOME/plausiden/config.toml`:
  ```toml
  host = "127.0.0.1"
  port = 3000
  theme = "auto"    # auto | light | dark | mono
  fact_inline = true
  history_size = 10000
  ```
- **Env overrides**: `PLAUSIDEN_HOST`, `PLAUSIDEN_PORT`, `PLAUSIDEN_NO_COLOR`.
- **Flags win** over env, env wins over config.

### Security
- **API tokens** read from `$XDG_CONFIG_HOME/plausiden/token` (chmod 600).
  Never echoed in the shell history.
- **`--no-color`** + **`NO_COLOR=1`** respected.
- **`$TERM`-safe**: no ANSI escapes if `$TERM` is unset or `dumb`.
- **Pipe-safe**: `plausiden chat "foo" | grep bar` works because output
  detects non-TTY and drops escapes + progress bars.
- **Signal handling**: `SIGPIPE` on stdout close → clean exit, not panic.

### Performance
- **Startup < 50ms** to first prompt (release build).
- **First token latency** < 200ms after Enter (network permitting —
  latency budget is just the CLI part).
- **Binary size** < 8MB (static-linked `musl`).

### Accessibility
- **Screen-reader friendly**: when `NVDA`/`JAWS` piping detected via
  `$ACCESSIBILITY_TOOLS=1` (user sets), suppress spinners + progress
  bars; print state as plain text.
- **Color-blind safe**: red/green verdict tokens also carry glyphs
  (`✓` Proved / `✗` Rejected / `?` Unknown / `~` Unreachable) so
  operators don't rely on color alone.

---

## Manual pre-release checklist

Run in: bash on Linux, zsh on macOS, PowerShell on Windows, `ssh -t`,
`tmux`, `screen`, and a plain dumb terminal.

- [ ] **Launch**: `plausiden` enters REPL, prompt shows, startup < 50ms.
- [ ] **Paste a multiline prompt**: detected as paste, confirms before sending.
- [ ] **Ctrl+C** mid-stream: interrupts, prompt returns; a second Ctrl+C
      in 2s exits cleanly.
- [ ] **Ctrl+D** on empty: `Bye.` + exit code 0.
- [ ] **Up-arrow**: walks history; repeat-lookup works after `/clear`.
- [ ] **Tab-complete** `/fa<tab>` → `/fact`; `/fact vol<tab>` → candidates.
- [ ] **`NO_COLOR=1 plausiden`**: no ANSI escapes in output.
- [ ] **`plausiden "hi" | head`**: pipe-safe (no progress bars / escapes).
- [ ] **`plausiden --json /drift`**: valid JSON on stdout, nothing on
      stderr except errors.
- [ ] **`--self-check`**: exits 0 with PASS summary.
- [ ] **Resize terminal** mid-session: tables reflow on next render.
- [ ] **`plausiden /fact concept:volcano`**: prints popover with ancestry
      arrays rendered as indented sub-trees.
- [ ] **`plausiden /verify concept:volcano`**: streams verdict with
      spinner while Lean4 runs; final `✓ Proved · h:<8>…` line.
- [ ] **Colorblind mode** (`$PLAUSIDEN_COLORBLIND=1`): glyphs on every
      verdict + verdict color uses blue/orange not green/red.
- [ ] **Screen reader mode** (`$ACCESSIBILITY_TOOLS=1`): no spinners.

---

## `--self-check` suite

The CLI runs these assertions on demand. Each returns PASS / FAIL.

```
$ plausiden --self-check
  PASS  config loads ($XDG_CONFIG_HOME/plausiden/config.toml)
  PASS  history file writable ($XDG_DATA_HOME/plausiden/history)
  PASS  backend reachable (http://127.0.0.1:3000/api/health 200 in 12ms)
  PASS  backend /api/health/extended schema parses
  PASS  TTY detection: stdout=interactive stderr=interactive
  PASS  color output enabled (COLORTERM=truecolor)
  PASS  SIGPIPE handler registered
  PASS  history has 247 entries, within 10000 cap
  FAIL  token file missing — run `plausiden /tokens issue`
  ---
  7/8 checks passed, 1 action needed
```

---

## Known-good Rust patterns

```rust
// ✅ TTY-aware color output (crossterm-based)
use std::io::{self, IsTerminal, Write};
fn color_if_tty() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

// ✅ Streaming chat from SSE — flush after every chunk
async fn stream_reply(mut stream: EventStream) -> anyhow::Result<()> {
    let mut out = io::stdout().lock();
    while let Some(event) = stream.next().await {
        if let Event::ChatChunk(s) = event? {
            out.write_all(s.as_bytes())?;
            out.flush()?;
        }
    }
    Ok(())
}

// ✅ Signal handling: Ctrl+C cancels, Ctrl+C twice exits
let mut ctrlc_at: Option<Instant> = None;
tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        if ctrlc_at.map_or(false, |t| t.elapsed() < Duration::from_secs(2)) {
            std::process::exit(0);
        }
        ctrlc_at = Some(Instant::now());
        cancel_current_turn();
    }
}
```

## Anti-patterns caught by `cli-lint.sh`

```rust
// ❌ println! on non-TTY produces ANSI escape garbage
println!("\x1b[32mok\x1b[0m");

// ❌ unwrap() on stdin/stdout — a broken pipe panics
stdout().write_all(...).unwrap();

// ❌ Storing API token in a command-line flag (leaks to ps output)
let token = env::args().nth(2).unwrap();  // arg visible to other users
// Read from $XDG_CONFIG_HOME/plausiden/token instead.

// ❌ Hardcoded 80-column assumption — breaks on ultra-wide terms.
let width = 80;  // use term_size or tty::size()
```
