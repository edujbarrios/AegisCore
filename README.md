

<h1 align="center">AegisCore</h1>

<p align="center"><strong>Local-first runtime to build, validate, and run declarative Codex-style skills.</strong></p>

<p align="center">
  <img src="docs/images/frontend-dark.png" alt="AegisCore skills frontend" />
</p>

## Why AegisCore

AegisCore gives you one local runtime for the full skill lifecycle:

- define skills as `.toml`, `.json`, or Markdown files
- validate and inspect them consistently
- run skills from CLI or HTTP
- control runtime tool access with explicit allowlists

It is designed for local development workflows where fast iteration and safer defaults matter.

## Features

- CLI + HTTP server for create, list, inspect, run, and delete operations
- Declarative skill formats validated with JSON Schema
- Per-skill tool allowlisting via `allowed_tools`
- Built-in runtime safety controls:
  - filesystem tools restricted to configured root and traversal-safe paths
  - `http_get` blocks localhost and private/loopback/link-local targets
  - `shell_command` disabled unless explicitly enabled
- Single-process full stack app (backend API + frontend UI)

## Quickstart (One Command Full Stack)

Prerequisite: Rust stable toolchain.

```bash
git clone https://github.com/edujbarrios/AegisCore.git
cd AegisCore
cargo run -- serve
```

Open `http://127.0.0.1:8787/`.

`cargo run -- serve` starts both:
- the HTTP API
- the built-in frontend served from `frontend/`

Optional: set API key when using OpenAI-compatible providers.

```bash
export OPENAI_API_KEY="..."
```

Optional first-run workspace initialization:

```bash
cargo run -- init
```

## CLI Usage

Create a skill:

```bash
cargo run -- create "Create a PDF summarizer"
```

List skills:

```bash
cargo run -- list
```

Inspect a skill:

```bash
cargo run -- inspect pdf_summarizer
```

Run a skill:

```bash
cargo run -- run pdf_summarizer --input input.json
```

Example `input.json`:

```json
{ "summary": "Fix skill markdown parsing", "issues": ["#123"] }
```

## Included Skills

The `skills/` directory includes ready-to-use Markdown skills:

- `github_commit` - draft a Conventional Commit message from diff/summary input
- `github_pr` - draft a GitHub PR title and body from diff/summary input
- `frontend-bug-reproducer` - convert frontend bug reports into a minimal repro + Playwright test + GitHub-ready report
- `paper_research` - research and curate academic papers for a topic
- `image_generation` - emit a complete image-generation spec with production-ready prompts
- `discussion` - guide interactive codebase discussion before planning or implementation
- `pdf_summaries` - summarize PDF-derived content into structured Markdown briefs

## Configuration

AegisCore reads `aegiscore.toml` by default (when present), or uses built-in defaults.
Use `--config <path>` to provide a custom config file.

Common settings:

- `llm.base_url`, `llm.model`, `llm.api_key_env`
- `runtime.max_tool_rounds`
- `runtime.fs_root`
- `runtime.allow_dangerous_tools`
- `runtime.max_read_bytes`, `runtime.max_write_bytes`, `runtime.http_max_bytes`, `runtime.http_timeout_ms`, `runtime.shell_timeout_ms`
- `server.host`, `server.port`

## Skill Format

Skills can be TOML/JSON documents, or Markdown with frontmatter.
Required fields: `name`, `version`, `description`, `author`, `license`, `system_prompt`, `allowed_tools`.

Minimal TOML:

```toml
name = "pdf_summarizer"
version = "0.1.0"
description = "Summarize a PDF into a short brief."
author = "Your Name"
license = "Apache-2.0"
system_prompt = "You are a helpful summarizer."
allowed_tools = ["read_file", "text_summarize"]
```

Minimal Markdown (YAML frontmatter + body as `system_prompt`):

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

Legacy `+++` TOML frontmatter in Markdown is still supported.

## HTTP API

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

## Development Validation

Before submitting changes, run:

```bash
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Open Source Project Guidelines

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.
- Review the [Code of Conduct](CODE_OF_CONDUCT.md).
- Report vulnerabilities through [SECURITY.md](SECURITY.md).

Issues and pull requests are welcome.

## License

Apache-2.0 (see `LICENSE`).
