## DeepSeek Agents (Rust)

Small, production-style Rust app that orchestrates two LLM agents end-to-end:

- **ProducerAgent**: generates a deliverable from a `TaskSpec` using `deepseek-chat`.
- **AuditorAgent**: evaluates that deliverable against acceptance criteria using `deepseek-reasoner`.

It also includes an interactive flow for collecting a `TaskSpec` and running the ProducerAgent, robust error handling, strict JSON prompting, and test coverage for the HTTP client.

## Features

- **Two-agent pipeline**: Producer → Auditor with separate models.
- **Strict JSON I/O**: agents prompt for structured JSON (`SolutionV1`, `ValidationV1`).
- **Interactive collection**: collect a task and save `solution.json` (`--console-producer`).
- **Retries and backoff**: transient HTTP failures are retried with exponential backoff (in the console client path).
- **Graceful cancellation (interactive loop)**: `Ctrl+C` cancellation in the interactive console loop.
- **Config via env/.env**: typed config with validation.
- **Logging**: `tracing` with `RUST_LOG` filter.
- **Tests**: WireMock-powered HTTP tests and async time control.

## Quick start

### Prerequisites

- **Rust (stable, 2024 edition)**
- A DeepSeek API key.

### Setup

1) Create a `.env` file in the project root (or export env vars):

```env
DEEPSEEK_API_KEY=sk-your-key
# Optional overrides
DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
DEEPSEEK_MODEL=deepseek-chat
DEEPSEEK_MAX_TOKENS=4096
DEEPSEEK_TEMPERATURE=0.7
DEEPSEEK_TIMEOUT=180
```

2) Build:

```bash
cargo build
```

3) Run (demo pipeline: Producer → Auditor):

```bash
cargo run --
```

This creates `out/solution.json` and `out/validation.json` and pretty-prints both to the console.

## CLI usage

The binary exposes flags via `clap`:

- **--task <PATH>**: path to a `TaskSpec` JSON file; if omitted, a demo `TaskSpec` is used.
- **--out-dir <PATH>**: output directory (default: `out`).
- **--console-producer**: interactive flow to collect a `TaskSpec` and run the ProducerAgent (writes only `solution.json`).

Examples:

```bash
# 1) Demo pipeline with built-in TaskSpec
cargo run --

# 2) Provide your own TaskSpec
cargo run -- --task /absolute/path/to/spec.json --out-dir /absolute/path/to/out

# 3) Interactive ProducerAgent (prompts, then saves solution.json)
cargo run -- --console-producer --out-dir /absolute/path/to/out

# Optional: verbose logging
RUST_LOG=debug cargo run -- --task /absolute/path/to/spec.json
```

## TaskSpec format

```json
{
  "task_id": "uuid-string",
  "goal": "Summarize the input into exactly 3 crisp bullet points",
  "input": "Some context to summarize",
  "acceptance_criteria": [
    "exactly 3 bullets",
    "<= 80 words total",
    "no marketing fluff"
  ],
  "deliverable_type": "text",
  "hints": "Be concise"
}
```

Valid `deliverable_type` values: `text`, `json`, `code`.

## Output artifacts

- **solution.json** (ProducerAgent) — `SolutionV1`
- **validation.json** (AuditorAgent) — `ValidationV1`

Example snippet (solution):

```json
{
  "schema_version": "solution_v1",
  "task_id": "...",
  "solution_id": "...",
  "model_used": { "name": "deepseek-chat", "temperature": 0.7 },
  "deliverable_type": "text",
  "deliverable": { "text": "- Bullet A\n- Bullet B\n- Bullet C" },
  "evidence": { "system_prompt": "...", "usage_note": null },
  "usage": { "prompt_tokens": 0, "completion_tokens": 0 },
  "created_at": "2024-01-01T00:00:00Z"
}
```

Example snippet (validation):

```json
{
  "schema_version": "validation_v1",
  "task_id": "...",
  "solution_id": "...",
  "verdict": "pass",
  "score": 0.98,
  "checks": [
    {
      "criterion": "exactly 3 bullets",
      "pass": true,
      "reason": "...",
      "severity": "minor",
      "suggested_fix": null
    }
  ],
  "suggested_rewrite": null,
  "model_used": { "name": "deepseek-reasoner", "temperature": 0.7 },
  "created_at": "2024-01-01T00:00:00Z"
}
```

## Architecture overview

- `src/main.rs`: CLI, logging, and pipeline orchestration (demo `TaskSpec` if `--task` absent). In pipeline mode: ProducerAgent → AuditorAgent; in `--console-producer` mode: interactive ProducerAgent only.
- `src/deepseek.rs`: HTTP client for DeepSeek Chat Completions. Handles retries/backoff, error mapping, and JSON parsing. Also provides `send_messages_raw` for strict JSON prompting.
- `src/agents/mod.rs`, `src/agents/producer.rs`, `src/agents/auditor.rs`: `Agent` trait and two implementations.
- `src/types.rs`: Strongly-typed schemas (`TaskSpec`, `SolutionV1`, `ValidationV1`, enums).
- `src/config.rs`: Loads and validates configuration from env/.env with sensible defaults.
- `src/console/*`: Interactive I/O and pretty console rendering of responses/artifacts.
- `src/orchestrator.rs`: Wires ProducerAgent and AuditorAgent, manages artifacts and console rendering in pipeline mode.

## Configuration

- **DEEPSEEK_API_KEY**: required.
- **DEEPSEEK_BASE_URL**: default `https://api.deepseek.com/v1`.
- **DEEPSEEK_MODEL**: default `deepseek-chat` (Producer). The Auditor uses `deepseek-reasoner` internally.
- **DEEPSEEK_MAX_TOKENS**: default `4096`.
- **DEEPSEEK_TEMPERATURE**: default `0.7`.
- **DEEPSEEK_TIMEOUT**: default `180` seconds.

## Development

- Run tests:

```bash
cargo test
```

- Useful env:

```bash
RUST_LOG=info    # or debug, trace
```

### Technologies used

- Runtime/concurrency: `tokio`
- HTTP: `reqwest` (rustls TLS)
- Serialization: `serde`, `serde_json`
- CLI: `clap`
- Logging: `tracing`, `tracing-subscriber`
- Config: `dotenv`
- UX: `colored`
- Time/UUID: `chrono`, `uuid`
- Errors: `anyhow`, `thiserror`
- Async traits: `async-trait`
- Tests: `wiremock`

## Notes and tips

- **Cancellation**: The interactive console loop supports `Ctrl+C` to cancel in-flight requests.
- **Backoff**: Transient server/network errors are retried with exponential backoff in the console client.
- **Security**: Never commit your `DEEPSEEK_API_KEY` to version control.

