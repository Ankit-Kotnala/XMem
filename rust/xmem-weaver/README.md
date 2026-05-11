# xmem-weaver

Standalone Rust implementation of the deterministic XMem weaver core.

This crate is intentionally not wired into the Python pipeline yet. It mirrors
the Python weaver's core contract:

- consume `JudgeResult`
- execute `ADD`, `UPDATE`, `DELETE`, `NOOP`
- return `WeaverResult`
- delegate storage writes through traits

The intended boundary is:

```text
Python orchestration / agents / prompts
        |
        v
Rust xmem-weaver deterministic execution
        |
        v
Vector store / graph store adapters
```

Current scope:

- vector domains: `profile`, `summary`, `image`
- graph domains: `temporal`
- code and snippet domains through vector execution hooks
- no Python imports
- no FastAPI or pipeline integration

Run tests from this directory:

```bash
cargo test
```
