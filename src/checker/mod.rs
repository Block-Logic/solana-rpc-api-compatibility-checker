mod get_health;

use crate::config::Config;
use crate::fixture::{MethodExpectation, RpcFixture};
use anyhow::{Context, Result};
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};

#[derive(Debug)]
pub struct CompatibilityReport {
    checks: Vec<CheckOutcome>,
}

impl CompatibilityReport {
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|check| !check.passed)
    }

    pub fn print_summary(&self) {
        for check in &self.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            println!(
                "[{status}] {} [{}] - {}",
                check.fixture_name, check.request_encoding, check.details
            );
        }

        let passed = self.checks.iter().filter(|check| check.passed).count();
        let failed = self.checks.len() - passed;
        println!();
        println!("Summary: {passed} passed, {failed} failed");
    }
}

#[derive(Debug)]
struct CheckOutcome {
    fixture_name: String,
    request_encoding: String,
    passed: bool,
    details: String,
}

#[derive(Debug)]
struct RequestThrottler {
    minimum_interval: Duration,
    last_request_started_at: Mutex<Option<Instant>>,
}

impl RequestThrottler {
    fn new(minimum_interval: Duration) -> Self {
        Self {
            minimum_interval,
            last_request_started_at: Mutex::new(None),
        }
    }

    async fn wait_for_turn(&self) {
        let mut guard = self.last_request_started_at.lock().await;

        if let Some(last_started_at) = *guard {
            let elapsed = last_started_at.elapsed();
            if elapsed < self.minimum_interval {
                sleep(self.minimum_interval - elapsed).await;
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
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .context("failed to construct HTTP client")?;
    let throttler = Arc::new(RequestThrottler::new(Duration::from_millis(
        config.minimum_request_interval_ms,
    )));
    let mut checks = Vec::new();

    for fixture in fixtures {
        for request_encoding in &fixture.request.encodings {
            let check = run_single_check(
                &client,
                throttler.clone(),
                config,
                fixture,
                request_encoding,
            )
            .await
            .with_context(|| {
                format!(
                    "fixture '{}' with request encoding '{}'",
                    fixture.name, request_encoding
                )
            });

            match check {
                Ok(details) => checks.push(CheckOutcome {
                    fixture_name: fixture.name.clone(),
                    request_encoding: request_encoding.clone(),
                    passed: true,
                    details,
                }),
                Err(error) => checks.push(CheckOutcome {
                    fixture_name: fixture.name.clone(),
                    request_encoding: request_encoding.clone(),
                    passed: false,
                    details: format!("{error:#}"),
                }),
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
    request_encoding: &str,
) -> Result<String> {
    match request_encoding {
        "json" => {}
        other => anyhow::bail!("request encoding '{other}' is not implemented yet"),
    }

    let request_id = format!("{}:{request_encoding}", fixture.method);
    let payload = JsonRpcRequest {
        jsonrpc: "2.0",
        id: request_id.clone(),
        method: &fixture.method,
        params: &fixture.request.params,
    };

    throttler.wait_for_turn().await;
    let response = client
        .post(&config.rpc_endpoint)
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await
        .context("RPC request failed")?;

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

    validate_response(fixture, &request_id, &response_data)
}

fn validate_response(
    fixture: &RpcFixture,
    request_id: &str,
    response: &HttpResponseData,
) -> Result<String> {
    if !response.status.is_success() {
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

    validate_charset(content_type, &fixture.expectation.transport.charset)?;

    let document: Value =
        serde_json::from_str(&response.body_text).context("response body was not valid JSON")?;

    assert_required_attributes(&document, &fixture.expectation.envelope.required_attributes)?;

    let jsonrpc_value = require_attribute(&document, "jsonrpc")?;
    let response_id_value = require_attribute(&document, "id")?;
    let result = require_attribute(&document, "result")?;

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

    if !fixture.expectation.envelope.allow_error
        && let Some(error) = document.get("error")
    {
        anyhow::bail!("response contained JSON-RPC error payload: {error}");
    }

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
        "getHealth" => Ok(get_health::validate),
        other => anyhow::bail!("no validator registered for RPC method '{other}'"),
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
                encodings: vec!["json".to_string()],
                params: Vec::new(),
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
            body_text: r#"{"jsonrpc":"2.0","result":"ok","id":"getHealth:json"}"#.to_string(),
        };

        let result = validate_response(&fixture(), "getHealth:json", &response);

        assert!(result.is_ok(), "expected validation to pass: {result:?}");
    }

    #[test]
    fn rejects_missing_charset() {
        let error = validate_charset("application/json", "utf-8").expect_err("missing charset");
        assert!(error.to_string().contains("none was provided"));
    }

    #[test]
    fn rejects_missing_required_result_field() {
        let response = HttpResponseData {
            status: reqwest::StatusCode::OK,
            content_type: Some("application/json; charset=utf-8".to_string()),
            body_text: r#"{"jsonrpc":"2.0","id":"getHealth:json"}"#.to_string(),
        };

        let error = validate_response(&fixture(), "getHealth:json", &response)
            .expect_err("missing result should fail");

        assert!(
            error
                .to_string()
                .contains("response was missing required 'result' field")
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
}
