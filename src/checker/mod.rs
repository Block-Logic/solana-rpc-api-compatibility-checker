mod get_account_info;
mod get_balance;
mod get_block;
mod get_block_commitment;
mod get_block_height;
mod get_block_production;
mod get_block_time;
mod get_blocks;
mod get_blocks_with_limit;
mod get_cluster_nodes;
mod get_epoch_info;
mod get_epoch_schedule;
mod get_fee_for_message;
mod get_first_available_block;
mod get_genesis_hash;
mod get_health;
mod get_highest_snapshot_slot;
mod get_identity;
mod get_inflation_governor;
mod get_inflation_rate;
mod get_inflation_reward;
mod get_largest_accounts;
mod get_latest_blockhash;
mod get_leader_schedule;
mod get_max_retransmit_slot;
mod get_max_shred_insert_slot;
mod get_minimum_balance_for_rent_exemption;
mod get_multiple_accounts;
mod get_program_accounts;
mod get_recent_performance_samples;
mod get_recent_prioritization_fees;
mod get_signature_statuses;
mod get_signatures_for_address;
mod get_slot;
mod get_slot_leader;
mod get_slot_leaders;
mod get_stake_minimum_delegation;
mod get_supply;
mod get_transaction;

use crate::config::Config;
use crate::fixture::{DynamicRequestParam, JsonRpcErrorExpectation, MethodExpectation, RpcFixture};
use anyhow::{Context, Result};
use reqwest::StatusCode;
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};

const GET_BLOCK_MINIMUM_REQUEST_INTERVAL_MS: u64 = 3_000;

#[derive(Debug)]
pub struct CompatibilityReport {
    checks: Vec<CheckOutcome>,
}

impl CompatibilityReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| matches!(check.status, CheckStatus::Failed))
    }

    pub fn print_summary(&self) {
        for check in &self.checks {
            let status = match check.status {
                CheckStatus::Passed => colorize_status_label("PASS", AnsiColor::Green),
                CheckStatus::Failed => colorize_status_label("FAIL", AnsiColor::Red),
                CheckStatus::Skipped => "SKIP".to_string(),
            };
            println!("[{status}] {} - {}", check.fixture_name, check.details);
        }

        let passed = self
            .checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Passed))
            .count();
        let failed = self
            .checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Failed))
            .count();
        let skipped = self
            .checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Skipped))
            .count();
        println!();
        println!("Summary: {passed} passed, {failed} failed, {skipped} skipped");
    }
}

#[derive(Debug, Clone, Copy)]
enum AnsiColor {
    Green,
    Red,
}

fn colorize_status_label(label: &str, color: AnsiColor) -> String {
    if !std::io::stdout().is_terminal() || std::env::var_os("NO_COLOR").is_some() {
        return label.to_string();
    }

    let color_code = match color {
        AnsiColor::Green => 32,
        AnsiColor::Red => 31,
    };

    format!("\x1b[{color_code}m{label}\x1b[0m")
}

#[derive(Debug)]
struct CheckOutcome {
    fixture_name: String,
    status: CheckStatus,
    details: String,
}

#[derive(Debug)]
enum CheckStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug)]
struct RequestThrottler {
    last_request_started_at: Mutex<Option<Instant>>,
}

impl RequestThrottler {
    fn new() -> Self {
        Self {
            last_request_started_at: Mutex::new(None),
        }
    }

    async fn wait_for_turn(&self, minimum_interval: Duration) {
        let mut guard = self.last_request_started_at.lock().await;

        if let Some(last_started_at) = *guard {
            let elapsed = last_started_at.elapsed();
            if elapsed < minimum_interval {
                sleep(minimum_interval - elapsed).await;
            }
        }

        *guard = Some(Instant::now());
    }
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: String,
    method: &'a str,
    params: &'a [Value],
}

#[derive(Debug)]
struct HttpResponseData {
    status: reqwest::StatusCode,
    content_type: Option<String>,
    body_text: String,
}

type MethodValidator = fn(&MethodExpectation, &Value) -> Result<String>;

pub async fn run_checks(config: &Config, fixtures: &[RpcFixture]) -> Result<CompatibilityReport> {
    run_checks_with_options(config, fixtures, false).await
}

pub async fn run_checks_with_options(
    config: &Config,
    fixtures: &[RpcFixture],
    show_failure_response: bool,
) -> Result<CompatibilityReport> {
    validate_health_gate_requirements(fixtures)?;

    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .context("failed to construct HTTP client")?;
    let throttler = Arc::new(RequestThrottler::new());
    let mut checks = Vec::new();
    let ordered_fixtures = order_fixtures(fixtures);
    let requires_health_gate = requires_health_gate(fixtures);
    let mut health_failed = false;

    for fixture in ordered_fixtures {
        if requires_health_gate && health_failed && fixture.method != "getHealth" {
            checks.push(CheckOutcome {
                fixture_name: fixture.name.clone(),
                status: CheckStatus::Skipped,
                details: "skipped because getHealth did not return ok".to_string(),
            });
            continue;
        }

        let check = run_single_check(
            &client,
            throttler.clone(),
            config,
            fixture,
            show_failure_response,
        )
        .await
        .with_context(|| format!("fixture '{}'", fixture.name));

        match check {
            Ok(details) => checks.push(CheckOutcome {
                fixture_name: fixture.name.clone(),
                status: CheckStatus::Passed,
                details,
            }),
            Err(error) => {
                if fixture.method == "getHealth" {
                    health_failed = true;
                }

                checks.push(CheckOutcome {
                    fixture_name: fixture.name.clone(),
                    status: CheckStatus::Failed,
                    details: format!("{error:#}"),
                });
            }
        }
    }

    Ok(CompatibilityReport { checks })
}

async fn run_single_check(
    client: &reqwest::Client,
    throttler: Arc<RequestThrottler>,
    config: &Config,
    fixture: &RpcFixture,
    show_failure_response: bool,
) -> Result<String> {
    let request_id = fixture.name.clone();
    let request_params = resolve_request_params(client, throttler.clone(), config, fixture).await?;
    let payload = JsonRpcRequest {
        jsonrpc: "2.0",
        id: request_id.clone(),
        method: &fixture.method,
        params: &request_params,
    };

    let response =
        send_rpc_request_with_retry(client, throttler, config, fixture, &payload).await?;

    let response_data = HttpResponseData {
        status: response.status(),
        content_type: response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        body_text: response
            .text()
            .await
            .context("failed to read response body")?,
    };

    match validate_response(fixture, &request_id, &response_data) {
        Ok(details) => Ok(details),
        Err(error) if show_failure_response => Err(error).context(format!(
            "full RPC response body: {}",
            response_data.body_text
        )),
        Err(error) => Err(error),
    }
}

async fn resolve_request_params(
    client: &reqwest::Client,
    throttler: Arc<RequestThrottler>,
    config: &Config,
    fixture: &RpcFixture,
) -> Result<Vec<Value>> {
    let mut params = fixture.request.params.clone();

    for dynamic_param in &fixture.request.dynamic_params {
        match dynamic_param {
            DynamicRequestParam::ProcessedSlot { index } => {
                let slot = fetch_processed_slot(client, throttler.clone(), config).await?;
                let params_len = params.len();
                let target = params.get_mut(*index).with_context(|| {
                    format!(
                        "dynamic processedSlot parameter index {} was outside params length {}",
                        index, params_len
                    )
                })?;
                *target = Value::from(slot);
            }
        }
    }

    Ok(params)
}

async fn fetch_processed_slot(
    client: &reqwest::Client,
    throttler: Arc<RequestThrottler>,
    config: &Config,
) -> Result<u64> {
    const MAX_ATTEMPTS: usize = 5;
    const TOO_MANY_REQUESTS_BACKOFF_MS: u64 = 10_000;

    let params = vec![serde_json::json!({ "commitment": "processed" })];
    let payload = JsonRpcRequest {
        jsonrpc: "2.0",
        id: "dynamic-getSlot-processed".to_string(),
        method: "getSlot",
        params: &params,
    };
    let request_interval = Duration::from_millis(config.minimum_request_interval_ms);

    for attempt in 1..=MAX_ATTEMPTS {
        throttler.wait_for_turn(request_interval).await;
        let response = client
            .post(&config.rpc_endpoint)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .context("dynamic getSlot processed request failed")?;

        if response.status() == StatusCode::TOO_MANY_REQUESTS && attempt < MAX_ATTEMPTS {
            sleep(Duration::from_millis(TOO_MANY_REQUESTS_BACKOFF_MS)).await;
            continue;
        }

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("failed to read dynamic getSlot processed response body")?;
        if !status.is_success() {
            anyhow::bail!(
                "dynamic getSlot processed expected an HTTP success status, received {status}"
            );
        }

        let document: Value = serde_json::from_str(&body_text)
            .context("dynamic getSlot processed response body was not valid JSON")?;
        if let Some(error) = document.get("error") {
            anyhow::bail!("dynamic getSlot processed returned JSON-RPC error payload: {error}");
        }

        return document
            .get("result")
            .and_then(Value::as_u64)
            .context("dynamic getSlot processed result was not a u64");
    }

    unreachable!("the retry loop always returns a response or error")
}

async fn send_rpc_request_with_retry(
    client: &reqwest::Client,
    throttler: Arc<RequestThrottler>,
    config: &Config,
    fixture: &RpcFixture,
    payload: &JsonRpcRequest<'_>,
) -> Result<reqwest::Response> {
    const MAX_ATTEMPTS: usize = 5;
    const TOO_MANY_REQUESTS_BACKOFF_MS: u64 = 10_000;
    let request_interval = minimum_request_interval_for_fixture(config, fixture);

    for attempt in 1..=MAX_ATTEMPTS {
        throttler.wait_for_turn(request_interval).await;
        let response = client
            .post(&config.rpc_endpoint)
            .header(CONTENT_TYPE, "application/json")
            .json(payload)
            .send()
            .await
            .with_context(|| format!("RPC request failed for method '{}'", fixture.method))?;

        if fixture.expectation.envelope.allow_error
            && response.status() == StatusCode::TOO_MANY_REQUESTS
        {
            return Ok(response);
        }

        if response.status() != StatusCode::TOO_MANY_REQUESTS || attempt == MAX_ATTEMPTS {
            return Ok(response);
        }

        sleep(Duration::from_millis(TOO_MANY_REQUESTS_BACKOFF_MS)).await;
    }

    unreachable!("the retry loop always returns a response or error")
}

fn minimum_request_interval_for_fixture(config: &Config, fixture: &RpcFixture) -> Duration {
    let minimum_interval_ms = match fixture.method.as_str() {
        "getBlock" => config
            .minimum_request_interval_ms
            .max(GET_BLOCK_MINIMUM_REQUEST_INTERVAL_MS),
        _ => config.minimum_request_interval_ms,
    };

    Duration::from_millis(minimum_interval_ms)
}

fn validate_response(
    fixture: &RpcFixture,
    request_id: &str,
    response: &HttpResponseData,
) -> Result<String> {
    let allows_rate_limit_error = fixture.expectation.envelope.allow_error
        && response.status == StatusCode::TOO_MANY_REQUESTS;

    if !response.status.is_success() && !allows_rate_limit_error {
        anyhow::bail!(
            "expected an HTTP success status, received {}",
            response.status
        );
    }

    let content_type = response
        .content_type
        .as_deref()
        .context("response did not include a Content-Type header")?;

    if !content_type.to_ascii_lowercase().starts_with(
        &fixture
            .expectation
            .transport
            .content_type_prefix
            .to_ascii_lowercase(),
    ) {
        anyhow::bail!(
            "expected Content-Type starting with '{}', received '{}'",
            fixture.expectation.transport.content_type_prefix,
            content_type
        );
    }

    if !(allows_rate_limit_error && !content_type.to_ascii_lowercase().contains("charset=")) {
        validate_charset(content_type, &fixture.expectation.transport.charset)?;
    }

    let document: Value =
        serde_json::from_str(&response.body_text).context("response body was not valid JSON")?;

    assert_required_attributes(&document, &fixture.expectation.envelope.required_attributes)?;

    let jsonrpc_value = require_attribute(&document, "jsonrpc")?;
    let response_id_value = require_attribute(&document, "id")?;

    let jsonrpc = jsonrpc_value
        .as_str()
        .context("jsonrpc field was not a string")?;
    if jsonrpc != fixture.expectation.envelope.jsonrpc_version {
        anyhow::bail!(
            "expected jsonrpc='{}', received '{jsonrpc}'",
            fixture.expectation.envelope.jsonrpc_version
        );
    }

    let response_id = response_id_value
        .as_str()
        .context("id field was not a string")?;
    if response_id != request_id {
        anyhow::bail!("expected id='{request_id}', received '{response_id}'");
    }

    if let Some(error) = document.get("error") {
        if !fixture.expectation.envelope.allow_error {
            anyhow::bail!("response contained JSON-RPC error payload: {error}");
        }

        let error_details =
            validate_expected_error(error, fixture.expectation.envelope.expected_error.as_ref())?;

        return Ok(format!(
            "status={} content-type='{}' {}",
            response.status, content_type, error_details
        ));
    }

    let result = require_attribute(&document, "result")?;
    let validator = validator_for_method(&fixture.method)?;
    let method_details = validator(&fixture.expectation.validator, result)?;

    Ok(format!(
        "status={} content-type='{}' {}",
        response.status, content_type, method_details
    ))
}

fn assert_required_attributes(document: &Value, required_attributes: &[String]) -> Result<()> {
    for attribute_name in required_attributes {
        require_attribute(document, attribute_name)?;
    }

    Ok(())
}

fn require_attribute<'a>(document: &'a Value, attribute_name: &str) -> Result<&'a Value> {
    document
        .get(attribute_name)
        .with_context(|| format!("response was missing required '{attribute_name}' field"))
}

fn validate_expected_error(
    error: &Value,
    expected_error: Option<&JsonRpcErrorExpectation>,
) -> Result<String> {
    let expected_error = expected_error
        .context("fixture allowed JSON-RPC errors but did not define expected_error")?;
    let error_object = error.as_object().context("error field was not an object")?;

    let actual_code = error_object
        .get("code")
        .and_then(Value::as_i64)
        .context("error.code field was not a signed integer")?;
    if actual_code != expected_error.code {
        anyhow::bail!(
            "expected error.code={}, received {}",
            expected_error.code,
            actual_code
        );
    }

    let actual_message = error_object
        .get("message")
        .and_then(Value::as_str)
        .context("error.message field was not a string")?;
    if actual_message != expected_error.message {
        anyhow::bail!(
            "expected error.message='{}', received '{}'",
            expected_error.message,
            actual_message
        );
    }

    Ok(format!(
        "error.code={} error.message='{}'",
        actual_code, actual_message
    ))
}

fn validate_charset(content_type: &str, expected_charset: &str) -> Result<()> {
    let lower = content_type.to_ascii_lowercase();
    let expected = expected_charset.to_ascii_lowercase();

    match lower.split(';').skip(1).find_map(|segment| {
        let trimmed = segment.trim();
        trimmed
            .strip_prefix("charset=")
            .map(|value| value.trim().to_ascii_lowercase())
    }) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => anyhow::bail!("expected charset '{expected}', received '{actual}'"),
        None => anyhow::bail!("expected Content-Type charset='{expected}', but none was provided"),
    }
}

fn validator_for_method(method: &str) -> Result<MethodValidator> {
    match method {
        "getAccountInfo" => Ok(get_account_info::validate),
        "getBalance" => Ok(get_balance::validate),
        "getBlock" => Ok(get_block::validate),
        "getBlockCommitment" => Ok(get_block_commitment::validate),
        "getBlockHeight" => Ok(get_block_height::validate),
        "getBlockTime" => Ok(get_block_time::validate),
        "getBlockProduction" => Ok(get_block_production::validate),
        "getBlocks" => Ok(get_blocks::validate),
        "getBlocksWithLimit" => Ok(get_blocks_with_limit::validate),
        "getClusterNodes" => Ok(get_cluster_nodes::validate),
        "getEpochInfo" => Ok(get_epoch_info::validate),
        "getEpochSchedule" => Ok(get_epoch_schedule::validate),
        "getFeeForMessage" => Ok(get_fee_for_message::validate),
        "getFirstAvailableBlock" => Ok(get_first_available_block::validate),
        "getGenesisHash" => Ok(get_genesis_hash::validate),
        "getHighestSnapshotSlot" => Ok(get_highest_snapshot_slot::validate),
        "getIdentity" => Ok(get_identity::validate),
        "getInflationGovernor" => Ok(get_inflation_governor::validate),
        "getInflationRate" => Ok(get_inflation_rate::validate),
        "getInflationReward" => Ok(get_inflation_reward::validate),
        "getLargestAccounts" => Ok(get_largest_accounts::validate),
        "getLeaderSchedule" => Ok(get_leader_schedule::validate),
        "getLatestBlockhash" => Ok(get_latest_blockhash::validate),
        "getMaxRetransmitSlot" => Ok(get_max_retransmit_slot::validate),
        "getMaxShredInsertSlot" => Ok(get_max_shred_insert_slot::validate),
        "getMinimumBalanceForRentExemption" => Ok(get_minimum_balance_for_rent_exemption::validate),
        "getHealth" => Ok(get_health::validate),
        "getMultipleAccounts" => Ok(get_multiple_accounts::validate),
        "getProgramAccounts" => Ok(get_program_accounts::validate),
        "getRecentPerformanceSamples" => Ok(get_recent_performance_samples::validate),
        "getRecentPrioritizationFees" => Ok(get_recent_prioritization_fees::validate),
        "getSignaturesForAddress" => Ok(get_signatures_for_address::validate),
        "getSignatureStatuses" => Ok(get_signature_statuses::validate),
        "getSlot" => Ok(get_slot::validate),
        "getSlotLeader" => Ok(get_slot_leader::validate),
        "getSlotLeaders" => Ok(get_slot_leaders::validate),
        "getStakeMinimumDelegation" => Ok(get_stake_minimum_delegation::validate),
        "getSupply" => Ok(get_supply::validate),
        "getTransaction" => Ok(get_transaction::validate),
        other => anyhow::bail!("no validator registered for RPC method '{other}'"),
    }
}

fn requires_health_gate(fixtures: &[RpcFixture]) -> bool {
    distinct_methods(fixtures).len() > 1
}

fn validate_health_gate_requirements(fixtures: &[RpcFixture]) -> Result<()> {
    let methods = distinct_methods(fixtures);

    if methods.len() > 1 && !methods.contains("getHealth") {
        anyhow::bail!(
            "multi-method runs must include a getHealth fixture so health can be checked first"
        );
    }

    Ok(())
}

fn distinct_methods(fixtures: &[RpcFixture]) -> BTreeSet<&str> {
    fixtures
        .iter()
        .map(|fixture| fixture.method.as_str())
        .collect()
}

fn order_fixtures(fixtures: &[RpcFixture]) -> Vec<&RpcFixture> {
    let mut ordered = fixtures.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| compare_fixtures(left, right));
    ordered
}

fn compare_fixtures(left: &RpcFixture, right: &RpcFixture) -> Ordering {
    match (left.method.as_str(), right.method.as_str()) {
        ("getHealth", "getHealth") => left.name.cmp(&right.name),
        ("getHealth", _) => Ordering::Less,
        (_, "getHealth") => Ordering::Greater,
        _ => left
            .method
            .cmp(&right.method)
            .then_with(|| left.name.cmp(&right.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{
        JsonRpcEnvelopeExpectation, RequestFixture, ResponseExpectation, TransportExpectation,
    };

    fn fixture() -> RpcFixture {
        RpcFixture {
            name: "getHealth returns ok".to_string(),
            method: "getHealth".to_string(),
            request: RequestFixture {
                params: Vec::new(),
                dynamic_params: Vec::new(),
            },
            expectation: ResponseExpectation {
                transport: TransportExpectation {
                    content_type_prefix: "application/json".to_string(),
                    charset: "utf-8".to_string(),
                },
                envelope: JsonRpcEnvelopeExpectation {
                    jsonrpc_version: "2.0".to_string(),
                    required_attributes: vec![
                        "jsonrpc".to_string(),
                        "result".to_string(),
                        "id".to_string(),
                    ],
                    allow_error: false,
                    expected_error: None,
                },
                validator: MethodExpectation::StringResult {
                    allowed_values: vec!["ok".to_string()],
                },
            },
        }
    }

    #[test]
    fn validates_successful_json_rpc_response() {
        let response = HttpResponseData {
            status: reqwest::StatusCode::OK,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","result":"ok","id":"getHealth returns ok"}"#.to_string(),
        };

        let result = validate_response(&fixture(), "getHealth returns ok", &response);

        assert!(result.is_ok(), "expected validation to pass: {result:?}");
    }

    #[test]
    fn rejects_missing_charset() {
        let error = validate_charset("application/json", "utf-8").expect_err("missing charset");
        assert!(error.to_string().contains("none was provided"));
    }

    #[test]
    fn uses_longer_minimum_request_interval_for_get_block() {
        let config = Config {
            rpc_endpoint: "https://example.com".to_string(),
            minimum_request_interval_ms: 2_000,
        };
        let mut fixture = fixture();
        fixture.method = "getBlock".to_string();

        let interval = minimum_request_interval_for_fixture(&config, &fixture);

        assert_eq!(
            interval,
            Duration::from_millis(GET_BLOCK_MINIMUM_REQUEST_INTERVAL_MS)
        );
    }

    #[test]
    fn preserves_configured_request_interval_for_other_methods() {
        let config = Config {
            rpc_endpoint: "https://example.com".to_string(),
            minimum_request_interval_ms: 2_000,
        };

        let interval = minimum_request_interval_for_fixture(&config, &fixture());

        assert_eq!(interval, Duration::from_millis(2_000));
    }

    #[test]
    fn rejects_missing_required_result_field() {
        let response = HttpResponseData {
            status: reqwest::StatusCode::OK,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","id":"getHealth returns ok"}"#.to_string(),
        };

        let error = validate_response(&fixture(), "getHealth returns ok", &response)
            .expect_err("missing result should fail");

        assert!(
            error
                .to_string()
                .contains("response was missing required 'result' field")
        );
    }

    #[test]
    fn validates_expected_json_rpc_error_response() {
        let mut fixture = fixture();
        fixture.name = "getBlock skipped slot".to_string();
        fixture.method = "getBlock".to_string();
        fixture.request.params = vec![
            serde_json::json!(410842412),
            serde_json::json!({
                "commitment": "finalized",
                "encoding": "json",
                "transactionDetails": "full",
                "maxSupportedTransactionVersion": 0,
                "rewards": true
            }),
        ];
        fixture.expectation.envelope.required_attributes =
            vec!["jsonrpc".to_string(), "error".to_string(), "id".to_string()];
        fixture.expectation.envelope.allow_error = true;
        fixture.expectation.envelope.expected_error = Some(JsonRpcErrorExpectation {
            code: -32007,
            message: "Slot 410842412 was skipped, or missing due to ledger jump to recent snapshot"
                .to_string(),
        });
        fixture.expectation.validator = MethodExpectation::BlockSnapshot {
            required_result_attributes: vec![],
            expected_result: serde_json::json!(null),
        };

        let response = HttpResponseData {
            status: reqwest::StatusCode::OK,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","error":{"code":-32007,"message":"Slot 410842412 was skipped, or missing due to ledger jump to recent snapshot"},"id":"getBlock skipped slot"}"#.to_string(),
        };

        let result = validate_response(&fixture, "getBlock skipped slot", &response);

        assert!(result.is_ok(), "expected validation to pass: {result:?}");
    }

    #[test]
    fn validates_expected_json_rpc_error_response_with_http_429() {
        let mut fixture = fixture();
        fixture.name = "getLargestAccounts finalized error".to_string();
        fixture.method = "getLargestAccounts".to_string();
        fixture.expectation.envelope.required_attributes =
            vec!["jsonrpc".to_string(), "error".to_string(), "id".to_string()];
        fixture.expectation.envelope.allow_error = true;
        fixture.expectation.envelope.expected_error = Some(JsonRpcErrorExpectation {
            code: 429,
            message: "Too many requests for a specific RPC call".to_string(),
        });
        fixture.expectation.validator = MethodExpectation::LargestAccounts {
            minimum_result_count: 1,
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["slot".to_string()],
            required_value_attributes: vec!["address".to_string(), "lamports".to_string()],
        };

        let response = HttpResponseData {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","error":{"code":429,"message":"Too many requests for a specific RPC call"},"id":"getLargestAccounts finalized error"}"#.to_string(),
        };

        let result = validate_response(&fixture, "getLargestAccounts finalized error", &response);

        assert!(result.is_ok(), "expected validation to pass: {result:?}");
    }

    #[test]
    fn validates_expected_json_rpc_error_response_with_http_429_and_no_charset() {
        let mut fixture = fixture();
        fixture.name = "getLargestAccounts finalized error".to_string();
        fixture.method = "getLargestAccounts".to_string();
        fixture.expectation.envelope.required_attributes =
            vec!["jsonrpc".to_string(), "error".to_string(), "id".to_string()];
        fixture.expectation.envelope.allow_error = true;
        fixture.expectation.envelope.expected_error = Some(JsonRpcErrorExpectation {
            code: 429,
            message: "Too many requests for a specific RPC call".to_string(),
        });
        fixture.expectation.validator = MethodExpectation::LargestAccounts {
            minimum_result_count: 1,
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["slot".to_string()],
            required_value_attributes: vec!["address".to_string(), "lamports".to_string()],
        };

        let response = HttpResponseData {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            content_type: Some("application/json".to_string()),
            body_text: r#"{"jsonrpc":"2.0","error":{"code":429,"message":"Too many requests for a specific RPC call"},"id":"getLargestAccounts finalized error"}"#.to_string(),
        };

        let result = validate_response(&fixture, "getLargestAccounts finalized error", &response);

        assert!(result.is_ok(), "expected validation to pass: {result:?}");
    }

    #[test]
    fn rejects_unexpected_json_rpc_error_message() {
        let mut fixture = fixture();
        fixture.name = "getBlock skipped slot".to_string();
        fixture.method = "getBlock".to_string();
        fixture.expectation.envelope.required_attributes =
            vec!["jsonrpc".to_string(), "error".to_string(), "id".to_string()];
        fixture.expectation.envelope.allow_error = true;
        fixture.expectation.envelope.expected_error = Some(JsonRpcErrorExpectation {
            code: -32007,
            message: "expected message".to_string(),
        });
        fixture.expectation.validator = MethodExpectation::BlockSnapshot {
            required_result_attributes: vec![],
            expected_result: serde_json::json!(null),
        };

        let response = HttpResponseData {
            status: reqwest::StatusCode::OK,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","error":{"code":-32007,"message":"actual message"},"id":"getBlock skipped slot"}"#.to_string(),
        };

        let error = validate_response(&fixture, "getBlock skipped slot", &response)
            .expect_err("should fail");

        assert!(
            error
                .to_string()
                .contains("expected error.message='expected message'")
        );
    }

    #[test]
    fn rejects_missing_required_attribute_from_fixture_list() {
        let error = assert_required_attributes(
            &serde_json::json!({"jsonrpc":"2.0","id":"getHealth:json"}),
            &[
                "jsonrpc".to_string(),
                "result".to_string(),
                "id".to_string(),
            ],
        )
        .expect_err("missing required attribute should fail");

        assert!(
            error
                .to_string()
                .contains("response was missing required 'result' field")
        );
    }

    #[test]
    fn sorts_get_health_before_other_methods() {
        let mut epoch_fixture = fixture();
        epoch_fixture.name = "getEpochInfo finalized".to_string();
        epoch_fixture.method = "getEpochInfo".to_string();
        epoch_fixture.expectation.validator = MethodExpectation::EpochInfo {
            required_result_attributes: vec![
                "absoluteSlot".to_string(),
                "blockHeight".to_string(),
                "epoch".to_string(),
                "slotIndex".to_string(),
                "slotsInEpoch".to_string(),
                "transactionCount".to_string(),
            ],
        };
        epoch_fixture.request.params = vec![serde_json::json!({"commitment":"finalized"})];

        let fixtures = [epoch_fixture, fixture()];
        let ordered = order_fixtures(&fixtures);

        assert_eq!(ordered[0].method, "getHealth");
        assert_eq!(ordered[1].method, "getEpochInfo");
    }

    #[test]
    fn rejects_multi_method_runs_without_get_health() {
        let fixtures = vec![
            RpcFixture {
                name: "getEpochInfo finalized".to_string(),
                method: "getEpochInfo".to_string(),
                request: RequestFixture {
                    params: vec![serde_json::json!({"commitment":"finalized"})],
                    dynamic_params: Vec::new(),
                },
                expectation: ResponseExpectation {
                    transport: TransportExpectation {
                        content_type_prefix: "application/json".to_string(),
                        charset: "utf-8".to_string(),
                    },
                    envelope: JsonRpcEnvelopeExpectation {
                        jsonrpc_version: "2.0".to_string(),
                        required_attributes: vec![
                            "jsonrpc".to_string(),
                            "result".to_string(),
                            "id".to_string(),
                        ],
                        allow_error: false,
                        expected_error: None,
                    },
                    validator: MethodExpectation::EpochInfo {
                        required_result_attributes: vec![
                            "absoluteSlot".to_string(),
                            "blockHeight".to_string(),
                            "epoch".to_string(),
                            "slotIndex".to_string(),
                            "slotsInEpoch".to_string(),
                            "transactionCount".to_string(),
                        ],
                    },
                },
            },
            RpcFixture {
                name: "getBalance sample".to_string(),
                method: "getBalance".to_string(),
                request: RequestFixture {
                    params: Vec::new(),
                    dynamic_params: Vec::new(),
                },
                expectation: ResponseExpectation {
                    transport: TransportExpectation {
                        content_type_prefix: "application/json".to_string(),
                        charset: "utf-8".to_string(),
                    },
                    envelope: JsonRpcEnvelopeExpectation {
                        jsonrpc_version: "2.0".to_string(),
                        required_attributes: vec![
                            "jsonrpc".to_string(),
                            "result".to_string(),
                            "id".to_string(),
                        ],
                        allow_error: false,
                        expected_error: None,
                    },
                    validator: MethodExpectation::EpochInfo {
                        required_result_attributes: vec![
                            "absoluteSlot".to_string(),
                            "blockHeight".to_string(),
                            "epoch".to_string(),
                            "slotIndex".to_string(),
                            "slotsInEpoch".to_string(),
                            "transactionCount".to_string(),
                        ],
                    },
                },
            },
        ];

        let error =
            validate_health_gate_requirements(&fixtures).expect_err("missing health fixture");

        assert!(
            error
                .to_string()
                .contains("must include a getHealth fixture")
        );
    }
}
