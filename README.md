# AegisCore

An open-source Rust runtime for managing declarative “skills” and executing OpenAI-compatible tool-calling with secure-by-default built-in tools.

Status: early-stage / experimental.

## What you get

- **CLI + HTTP server** to create, list, inspect, run, and delete skills.
- **Declarative skills** stored as `skills/*.toml` or `skills/*.json` (validated with JSON Schema).
- **Tool allowlisting** per-skill (`allowed_tools`) with OpenAI-style function tool definitions.
- **Safe defaults**:
  - Filesystem tools are sandboxed to a configurable root and block `..` traversal and absolute paths.
  - `http_get` blocks localhost and private/loopback/link-local IP ranges.
  - `shell_command` exists but is disabled unless explicitly enabled.

## Quickstart

Prereqs: Rust toolchain (stable).

Initialize a workspace (creates `docs/`, `examples/`, `skills/`, `modules/`, plus `aegiscore.toml` and `.env.example` if missing):

```bash
cargo run -- init
```

Optionally configure an OpenAI-compatible API key:

```bash
export OPENAI_API_KEY="..."
```

Create a skill:

```bash
cargo run -- create "Create a PDF summarizer"
```

List and inspect skills:

```bash
cargo run -- list
cargo run -- inspect pdf_summarizer
```

Run a skill (if no LLM credentials are configured, AegisCore falls back to a local provider):

```bash
cargo run -- run pdf_summarizer --input input.json
```

Start the HTTP server:

```bash
cargo run -- serve
```

## Configuration

By default, AegisCore loads `aegiscore.toml` if present; otherwise it uses built-in defaults. You can pass an explicit config path with `--config <path>`.

Key settings:

- `llm.base_url`, `llm.model`, `llm.api_key_env`
- `runtime.max_tool_rounds`
- `runtime.fs_root` (filesystem sandbox root for `read_file` / `write_file` / `list_files`)
- `runtime.allow_dangerous_tools` (enables `shell_command`)
- size/time limits: `runtime.max_read_bytes`, `runtime.max_write_bytes`, `runtime.http_max_bytes`, `runtime.http_timeout_ms`, `runtime.shell_timeout_ms`
- `server.host`, `server.port`

## Skill format

Skills are TOML/JSON documents with the following required fields:
`name`, `version`, `description`, `author`, `license`, `system_prompt`, `allowed_tools`.

Minimal TOML example:

```toml
name = "pdf_summarizer"
version = "0.1.0"
description = "Summarize a PDF into a short brief."
author = "Your Name"
license = "Apache-2.0"
system_prompt = "You are a helpful summarizer."
allowed_tools = ["read_file", "text_summarize"]
```

## HTTP API (server mode)

When running `cargo run -- serve`, the server exposes:

- `GET /health`
- `GET /skills`
- `GET /skills/{name}`
- `POST /skills/create`
- `DELETE /skills/{name}`
- `POST /skills/{name}/run`
- `GET /tools`
- `GET /modules`

Example:

```bash
curl -s http://127.0.0.1:8787/health
```

## Security notes

AegisCore is intended to run as a local tool with conservative defaults, but it is **not** a hardened sandbox:

- `shell_command` is disabled unless you set `runtime.allow_dangerous_tools=true`.
- Filesystem access is restricted to `runtime.fs_root`, but you should still treat skills as code-like inputs.
- Network requests via `http_get` are limited and block local/private targets to reduce SSRF risk.

If you discover a security issue, please follow `SECURITY.md`.

## Contributing

See `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.

## License

Apache-2.0 (see `LICENSE`).
