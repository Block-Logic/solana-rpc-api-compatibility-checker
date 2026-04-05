# AGENTS.md

## Purpose

This repository validates Solana JSON-RPC behavior against the public API docs at:

- `https://solana.com/docs/rpc`

The tool loads local fixtures, sends live JSON-RPC requests to `RPC_ENDPOINT`, and validates the
transport, JSON-RPC envelope, and method-specific response shape.

## Environment

- Primary runtime: Rust
- Entry point: `src/main.rs`
- Config loader: `src/config.rs`
- Fixture schema: `src/fixture.rs`
- Shared checker runner: `src/checker/mod.rs`
- Method validators: `src/checker/get_*.rs`
- Fixtures: `fixtures/rpc/<method>/`

The repo expects `.env` to define `RPC_ENDPOINT`.

## Core workflow

## Branch policy

Before adding or updating any RPC method, ensure the repo is on a feature branch.

- If already on a feature branch, continue there.
- If on `main`, create a new branch first.
- Use the naming convention `methodName-YYYMMDD` where `methodName` is the name of the RPC method.
- For today's date, use `20260404`.
- Example: `getBalance-20260404`

When adding or updating an RPC method:

0. Ensure the repo is on a feature branch first.
   If not, create one using the naming convention `methodName-YYYMMDD`.
1. Read the official Solana RPC docs for that method.
2. Probe the live endpoint in `.env` to confirm the current response shape.
3. Add or update fixtures under `fixtures/rpc/<method>/`.
4. Extend `MethodExpectation` in `src/fixture.rs` if the method needs a new validator shape.
5. Add a method validator in `src/checker/get_<method>.rs`.
6. Register the validator in `src/checker/mod.rs`.
7. Update `README.md` if the supported methods list or workflow changes.
8. Run tests and a method-specific live check before finishing.

## Validation rules

- Prefer structural validators for responses that are expected to drift between runs.
- Use exact snapshot validation only when the response is intentionally pinned and stable enough.
- If a value is known to change between runs, validate the shape and invariant instead of exact equality.
- For dynamic numeric fields, prefer checks like `> 0` when that matches the intent.
- For JSON-RPC error fixtures, pin the observed `error.code` and `error.message`.

## Fixture conventions

- One fixture file per concrete RPC scenario.
- Keep filenames descriptive and stable.
- If the user requests a prefix convention for filenames, follow it exactly.
- Put request parameters in `request.params`.
- Put transport and JSON-RPC expectations in `expectation.transport` and `expectation.envelope`.
- Put method-specific assertions in `expectation.validator`.
- If a fixture is intended to validate an error response, use:
  - `required_attributes: ["jsonrpc", "error", "id"]`
  - `allow_error: true`
  - `expected_error`

## Live endpoint expectations

Before sending work back, validate against:

- `https://api.mainnet-beta.solana.com`

That is an explicit project expectation from the repository README.

If the current `.env` points somewhere else during development, it is still useful for exploration,
but final fixture validity should be checked against public mainnet unless the user says otherwise.

## Commands

Useful commands:

```bash
cargo test
cargo run
cargo run -- --method getHealth
cargo run -- --method <rpc-method>
cargo run -- --method <rpc-method> --show-failure-response
```

The app prints the active endpoint on startup:

```text
Running against RPC_ENDPOINT=...
```

## Implementation notes

- The runner throttles requests to stay under 2 requests per second.
- Multi-method runs require a `getHealth` fixture and will skip later methods if health fails.
- The checker recursively loads all `.json` fixtures in `fixtures/rpc`.
- The `--show-failure-response` flag is the fastest way to inspect live response drift.

## Editing guidance

- Keep changes narrow and consistent with the current fixture schema.
- Add unit tests for new validator behavior.
- Prefer extending existing patterns over inventing a separate framework.
- If live RPC behavior has drifted, refresh fixtures from live responses rather than weakening validation without reason.
- After adding a new validator for an RPC method, update the "Current Methods" and "Project Layout" sections of README.md, and sort the methods in alphabetical order.

## Done criteria

A task is usually complete when:

- the new or updated method has fixtures
- the validator is implemented and registered
- `cargo test` passes
- `cargo run -- --method <rpc-method>` passes against the intended endpoint
