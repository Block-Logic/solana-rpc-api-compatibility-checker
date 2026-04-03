# solana-rpc-api-compatibility-checker
Check a Solana RPC endpoint to check conformity with the standard API spec described at https://solana.com/docs/rpc.

Usage:
`cp dot_env_example.txt .env` then edit .env as desired.

## Current scaffold

This repository now includes an initial Rust scaffold for validating the `getHealth` JSON-RPC
method against a configured `RPC_ENDPOINT`, with a generalized fixture schema for future methods.

The checker currently:

- loads `.env` with `RPC_ENDPOINT`
- recursively reads local fixtures from `fixtures/rpc`
- sends JSON-RPC requests to the configured endpoint
- enforces a minimum 2000 ms delay between requests so the process stays comfortably under 2 requests/second on public RPC endpoints
- validates the HTTP success status, `Content-Type`, charset, and JSON-RPC envelope
- dispatches each fixture to a method-specific validator after the shared checks pass
- starts multi-method runs with `getHealth` and skips later methods if health is not `ok`

## Run the checker

```bash
cargo run
```

To run only one RPC method's fixtures:

```bash
cargo run -- --method getHealth
```

If the endpoint behaves as expected, the checker prints a passing summary. If a validation fails, the
process exits with a non-zero status and prints the failure details.

## Fixture format

Each fixture file is a local JSON document that describes one RPC method scenario. The initial
`getHealth` fixture looks like this:

```json
{
  "name": "getHealth returns ok",
  "method": "getHealth",
  "request": {
    "params": []
  },
  "expectation": {
    "transport": {
      "content_type_prefix": "application/json",
      "charset": "utf-8"
    },
    "envelope": {
      "jsonrpc_version": "2.0",
      "required_attributes": ["jsonrpc", "result", "id"]
    },
    "validator": {
      "kind": "stringResult",
      "allowed_values": ["ok"]
    }
  }
}
```

The top-level shape is now method-agnostic:

- `request.params` holds the JSON-RPC params for the scenario
- `expectation.transport` covers shared HTTP checks
- `expectation.envelope` covers shared JSON-RPC checks
- `expectation.envelope.required_attributes` lists the response fields that must be present
- `expectation.envelope.expected_error` can pin a JSON-RPC error code and message for cases such as skipped slots or unavailable historical data
- `expectation.validator` holds the method-specific assertion payload, including required fields inside `result` when needed

Each fixture now represents one concrete RPC scenario. That maps more cleanly to methods like
`getEpochInfo`, whose request config supports `commitment` and `minContextSlot` but not an encoding
parameter in the request object. Methods that need multiple encoding-style scenarios can express them
as separate fixtures with different `params`.

## Current methods

- `getHealth`: validates the health string response and is used as the gate for multi-method runs
- `getEpochInfo`: validates the documented epoch info object for `processed`, `confirmed`, and `finalized` commitments
- `getTransaction`: validates exact transaction snapshots for supported response formats such as `json`, `jsonParsed`, `base64`, and `base58`
  Snapshot fixtures can pin `meta` fields and `logMessages` exactly for specific signatures
- `getBlock`: validates exact block snapshots for supported response formats such as `json`, `jsonParsed`, `base64`, and `base58`
  It can also validate expected JSON-RPC errors for skipped or unavailable slots

## Project layout

- `src/config.rs`: loads environment configuration
- `src/fixture.rs`: parses recursive, method-agnostic RPC fixtures
- `src/checker/mod.rs`: shared runner, throttling, transport checks, and validator dispatch
- `src/checker/get_health.rs`: method-specific validation for `getHealth`
- `src/checker/get_epoch_info.rs`: method-specific validation for `getEpochInfo`
- `src/checker/get_transaction.rs`: method-specific validation for `getTransaction`
- `src/checker/get_block.rs`: method-specific validation for `getBlock`
- `fixtures/rpc/getHealth/`: first fixture set for `getHealth`
- `fixtures/rpc/getEpochInfo/`: commitment-specific fixtures for `getEpochInfo`
- `fixtures/rpc/getTransaction/`: signature-specific fixtures for `getTransaction`
- `fixtures/rpc/getBlock/`: block-specific fixtures for `getBlock`

## Next steps

The scaffold is still intentionally small, but the fixture schema is now broad enough that the next
RPC methods should mostly require:

1. adding fixture files under `fixtures/rpc/<method>/`
2. registering a validator for the method
3. teaching that validator how to interpret `expectation.validator`
