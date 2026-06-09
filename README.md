# AegisCore

AegisCore is the rust based software to display and interact with codex skills used by Eduardo J. Barrios.

Status: early-stage / experimental.

## What you get

- **CLI + HTTP server** to create, list, inspect, run, and delete skills.
- **Declarative skills** stored as `skills/*.toml`, `skills/*.json`, or `skills/*.md` (validated with JSON Schema).
- **Tool allowlisting** per-skill (`allowed_tools`) with OpenAI-style function tool definitions.
- **Safe defaults**:
  - Filesystem tools are sandboxed to a configurable root and block `..` traversal and absolute paths.
  - `http_get` blocks localhost and private/loopback/link-local IP ranges.
  - `shell_command` exists but is disabled unless explicitly enabled.

## Quickstart

Prereqs: Rust toolchain (stable).

Clone the repo:

```bash
git clone https://github.com/edujbarrios/AegisCore.git
cd AegisCore
```

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

This repo also includes a couple of ready-to-use Markdown skills in `skills/`:
- `github_commit` - drafts a Conventional Commit message from a diff/summary input
- `github_pr` - drafts a GitHub PR title + body from a diff/summary input
- `frontend-bug-reproducer` - turns frontend bug reports into a minimal repro + Playwright test + GitHub-ready report
- `paper_research` - researches and curates academic papers for a topic using public scholarly APIs
- `image_generation` - outputs a complete, parameterized image-generation spec with production-ready prompts
- `discussion` - guides interactive codebase discussions before planning or implementation

`github_commit` and `github_pr` emit **strict JSON**. The generated text is ready to paste: `github_commit.commit.full_message` is a complete Conventional Commit message, and `github_pr.pr.body_markdown` is a Markdown PR description.

`frontend-bug-reproducer` emits a **single GitHub-ready Markdown report** (inside the `output` string).
`image_generation` emits **strict JSON** with prompt, negative prompt, alternatives, and full generation parameters.

Example input (`input.json`):

```json
{ "summary": "Fix skill markdown parsing", "issues": ["#123"] }
```

Start the HTTP server:

```bash
cargo run -- serve
```

Then open the skills frontend at `http://127.0.0.1:8787/`.

## Skills Frontend

The built-in dark themed TypeScript frontend renders skills as interactive cards with quick actions to:
- open full skill details
- copy the system prompt
- copy full skill JSON for paste-ready sharing

![AegisCore skills frontend](docs/images/frontend-dark.png)

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

Skills are TOML/JSON documents (or Markdown with frontmatter) with the following required fields:
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

Minimal Markdown example (YAML frontmatter + body). The Markdown body is used as the `system_prompt` that gets sent straight to the agent as its `system` message:

```md
---
name: pdf_summarizer
version: "0.1.0"
description: Summarize a PDF into a short brief.
author: Your Name
license: Apache-2.0
allowed-tools:
  - read_file
  - text_summarize
---
You are a helpful summarizer.
```

Legacy `+++` TOML frontmatter in Markdown skills is also supported for backward compatibility.

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
