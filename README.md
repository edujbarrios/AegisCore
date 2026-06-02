# AegisCore

My Personal Agent Skills and Tools Handler

## What it is

AegisCore is a modular Rust runtime for managing declarative skills and executing OpenAI-compatible tool calling with safe defaults.

## Quickstart

Initialize a workspace:

```bash
cargo run -- init
```

Create a skill:

```bash
cargo run -- create "Create a PDF summarizer"
```

List and inspect skills:

```bash
cargo run -- list
cargo run -- inspect create_a_pdf_summarizer
```

Run a skill (uses OpenAI-compatible provider when `OPENAI_API_KEY` is set; otherwise falls back to a local echo provider):

```bash
cargo run -- run create_a_pdf_summarizer --input input.json
```

Start the HTTP server:

```bash
cargo run -- serve
```

## Configuration

Default config lives in `aegiscore.toml`.

- `runtime.allow_dangerous_tools`: keeps `shell_command` disabled by default
- `runtime.fs_root`: filesystem sandbox root for `read_file`, `write_file`, and `list_files`
- `runtime.max_tool_rounds`: maximum tool-calling loop iterations

## Security defaults

- Filesystem sandboxing with path traversal protection
- `http_get` blocks localhost and obvious private/loopback IPs
- `shell_command` is disabled unless explicitly enabled in config

## Commit roadmap (recommended)

- `feat: initialize workspace and config loader`
- `feat: add tool registry and built-in tools`
- `feat: add skill schema validation and skill store`
- `feat: add provider abstraction and OpenAI provider`
- `feat: add runtime tool-calling loop`
- `feat: add HTTP server endpoints`
- `docs: expand README and contribution/security docs`
