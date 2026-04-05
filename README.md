# solana-rpc-api-compatibility-checker
Check a Solana RPC endpoint to check conformity with the standard API spec described at https://solana.com/docs/rpc.

Note that I am using Codex for this project. Please validate that all PRs pass tests & all fixtures are valid for 'https://api.mainnet-beta.solana.com' before sending them to me.

Usage:
`cp dot_env_example.txt .env` then edit .env as desired. Review the Run the checker section below for more instructions. 

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

To print the full RPC response body for any failed validation:

```bash
cargo run -- --method getProgramAccounts --show-failure-response
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
- `getEpochSchedule`: validates the documented epoch schedule object shape without pinning cluster-specific values
- `getFeeForMessage`: validates the full response shape for a base64 message, including `context` and a `value` that may be either `u64` or `null`
- `getFirstAvailableBlock`: validates the full JSON-RPC response shape and pins the returned first available block to `0`
- `getGenesisHash`: validates the full JSON-RPC response shape and checks that the returned genesis hash is a non-empty string
- `getHighestSnapshotSlot`: validates the full JSON-RPC response shape for snapshot metadata, including `full` and an `incremental` value that may be `u64` or `null`
- `getBalance`: validates the documented balance response for finalized commitment and asserts the returned lamport balance is greater than zero
- `getBlockCommitment`: validates exact block commitment snapshots for stable slots such as `2`
- `getBlockHeight`: validates the documented finalized block height response and asserts the returned value is greater than zero
- `getBlockTime`: validates exact stable block-time values for fixed finalized slots such as `100000000`
- `getBlockProduction`: validates the finalized block production response shape for a specific validator identity without pinning the changing counters
- `getBlocks`: validates exact stable slot-list snapshots for fixed finalized ranges such as `2..10`
- `getBlocksWithLimit`: validates exact stable slot-list snapshots for fixed finalized start/limit queries such as `2` with limit `10`
- `getClusterNodes`: validates the dynamic cluster-nodes response shape and checks the first node entry has the documented fields and value types
- `getAccountInfo`: validates structural single-account responses for supported finalized encodings such as `base58`, `base64`, `base64+zstd`, and `jsonParsed`
  The validator checks `result.context`, `result.value`, and the returned account data shape for the selected encoding
- `getMultipleAccounts`: validates structural multi-account responses for supported finalized encodings such as `base58`, `base64`, `base64+zstd`, and `jsonParsed`
  The validator checks `result.context`, preserves account order, and validates each returned account entry
- `getTransaction`: validates exact transaction snapshots for supported response formats such as `json`, `jsonParsed`, `base64`, and `base58`
  Snapshot fixtures can pin `meta` fields and `logMessages` exactly for specific signatures
- `getBlock`: validates exact block snapshots for supported response formats such as `json`, `jsonParsed`, `base64`, and `base58`
  It can also validate expected JSON-RPC errors for skipped or unavailable slots
- `getProgramAccounts`: validates structural account-list responses for live stake-program queries using finalized commitment and supported encodings such as `base64`, `base64+zstd`, and `jsonParsed`
  The validator asserts account count is greater than zero and checks the shape of each returned account entry

## Project layout

- `src/config.rs`: loads environment configuration
- `src/fixture.rs`: parses recursive, method-agnostic RPC fixtures
- `src/checker/mod.rs`: shared runner, throttling, transport checks, and validator dispatch
- `src/checker/get_health.rs`: method-specific validation for `getHealth`
- `src/checker/get_epoch_info.rs`: method-specific validation for `getEpochInfo`
- `src/checker/get_epoch_schedule.rs`: method-specific validation for `getEpochSchedule`
- `src/checker/get_fee_for_message.rs`: method-specific validation for `getFeeForMessage`
- `src/checker/get_first_available_block.rs`: method-specific validation for `getFirstAvailableBlock`
- `src/checker/get_genesis_hash.rs`: method-specific validation for `getGenesisHash`
- `src/checker/get_highest_snapshot_slot.rs`: method-specific validation for `getHighestSnapshotSlot`
- `src/checker/get_balance.rs`: method-specific validation for `getBalance`
- `src/checker/get_block_commitment.rs`: method-specific validation for `getBlockCommitment`
- `src/checker/get_block_height.rs`: method-specific validation for `getBlockHeight`
- `src/checker/get_block_time.rs`: method-specific validation for `getBlockTime`
- `src/checker/get_block_production.rs`: method-specific validation for `getBlockProduction`
- `src/checker/get_blocks.rs`: method-specific validation for `getBlocks`
- `src/checker/get_blocks_with_limit.rs`: method-specific validation for `getBlocksWithLimit`
- `src/checker/get_cluster_nodes.rs`: method-specific validation for `getClusterNodes`
- `src/checker/get_account_info.rs`: method-specific validation for `getAccountInfo`
- `src/checker/get_multiple_accounts.rs`: method-specific validation for `getMultipleAccounts`
- `src/checker/get_transaction.rs`: method-specific validation for `getTransaction`
- `src/checker/get_block.rs`: method-specific validation for `getBlock`
- `src/checker/get_program_accounts.rs`: method-specific validation for `getProgramAccounts`
- `fixtures/rpc/getHealth/`: first fixture set for `getHealth`
- `fixtures/rpc/getEpochInfo/`: commitment-specific fixtures for `getEpochInfo`
- `fixtures/rpc/getEpochSchedule/`: structural fixtures for `getEpochSchedule`
- `fixtures/rpc/getFeeForMessage/`: message-specific fixtures for `getFeeForMessage`
- `fixtures/rpc/getFirstAvailableBlock/`: exact-value fixtures for `getFirstAvailableBlock`
- `fixtures/rpc/getGenesisHash/`: structural fixtures for `getGenesisHash`
- `fixtures/rpc/getHighestSnapshotSlot/`: structural fixtures for `getHighestSnapshotSlot`
- `fixtures/rpc/getBalance/`: account-specific fixtures for `getBalance`
- `fixtures/rpc/getBlockCommitment/`: block-specific fixtures for `getBlockCommitment`
- `fixtures/rpc/getBlockHeight/`: finalized fixtures for `getBlockHeight`
- `fixtures/rpc/getBlockTime/`: slot-specific fixtures for `getBlockTime`
- `fixtures/rpc/getBlockProduction/`: identity-specific fixtures for `getBlockProduction`
- `fixtures/rpc/getBlocks/`: finalized range fixtures for `getBlocks`
- `fixtures/rpc/getBlocksWithLimit/`: finalized start-and-limit fixtures for `getBlocksWithLimit`
- `fixtures/rpc/getClusterNodes/`: structural fixtures for `getClusterNodes`
- `fixtures/rpc/getAccountInfo/`: account-specific fixtures for `getAccountInfo`
- `fixtures/rpc/getMultipleAccounts/`: account-list fixtures for `getMultipleAccounts`
- `fixtures/rpc/getTransaction/`: signature-specific fixtures for `getTransaction`
- `fixtures/rpc/getBlock/`: block-specific fixtures for `getBlock`
- `fixtures/rpc/getProgramAccounts/`: encoding-specific fixtures for `getProgramAccounts`

## Next steps

The scaffold is still intentionally small, but the fixture schema is now broad enough that the next
RPC methods should mostly require:

1. adding fixture files under `fixtures/rpc/<method>/`
2. registering a validator for the method
3. teaching that validator how to interpret `expectation.validator`
4. Expand test coverage with more examples, especially error cases to confirm the correct error codes are received.
