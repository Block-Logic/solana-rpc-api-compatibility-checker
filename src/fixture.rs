use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct RpcFixture {
    pub name: String,
    pub method: String,
    pub request: RequestFixture,
    pub expectation: ResponseExpectation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestFixture {
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
    #[serde(default)]
    pub dynamic_params: Vec<DynamicRequestParam>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DynamicRequestParam {
    ProcessedSlot { index: usize },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseExpectation {
    pub transport: TransportExpectation,
    pub envelope: JsonRpcEnvelopeExpectation,
    pub validator: MethodExpectation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransportExpectation {
    pub content_type_prefix: String,
    pub charset: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcEnvelopeExpectation {
    pub jsonrpc_version: String,
    #[serde(default = "default_required_response_attributes")]
    pub required_attributes: Vec<String>,
    #[serde(default)]
    pub allow_error: bool,
    #[serde(default)]
    pub expected_error: Option<JsonRpcErrorExpectation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcErrorExpectation {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MethodExpectation {
    StringResult {
        allowed_values: Vec<String>,
    },
    BlockCommitment {
        required_result_attributes: Vec<String>,
        expected_commitment: serde_json::Value,
    },
    BlockTime {
        expected_value: u64,
    },
    BlocksSnapshot {
        expected_result: serde_json::Value,
    },
    BlocksWithLimitSnapshot {
        expected_result: serde_json::Value,
    },
    BlockProduction {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
        required_range_attributes: Vec<String>,
        expected_identity: String,
    },
    ClusterNodes {
        minimum_result_count: usize,
        required_node_attributes: Vec<String>,
        required_string_attributes: Vec<String>,
        nullable_string_attributes: Vec<String>,
        required_u64_attributes: Vec<String>,
    },
    BlockHeight,
    EpochInfo {
        required_result_attributes: Vec<String>,
    },
    EpochSchedule {
        required_result_attributes: Vec<String>,
    },
    FeeForMessage {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
    },
    FirstAvailableBlock {
        expected_value: u64,
    },
    GenesisHash,
    Identity {
        required_result_attributes: Vec<String>,
    },
    InflationGovernor {
        required_result_attributes: Vec<String>,
        expected_result: serde_json::Value,
    },
    InflationRate {
        required_result_attributes: Vec<String>,
    },
    InflationReward {
        expected_result_length: usize,
        required_reward_attributes: Vec<String>,
    },
    LargestAccounts {
        minimum_result_count: usize,
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
    },
    LeaderSchedule {
        minimum_validator_count: usize,
    },
    HighestSnapshotSlot {
        required_result_attributes: Vec<String>,
    },
    LatestBlockhash {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
    },
    Slot,
    SlotLeader,
    SlotLeaders {
        expected_result_length: usize,
    },
    StakeMinimumDelegation {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
    },
    Supply {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
    },
    TokenAccountBalance {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
    },
    MaxRetransmitSlot,
    MaxShredInsertSlot,
    MinimumBalanceForRentExemption {
        expected_value: u64,
    },
    Balance {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        #[serde(default)]
        expected_value: Option<u64>,
    },
    AccountInfo {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
        expected_value_attributes: serde_json::Value,
        expected_owner: String,
        expected_data_encoding: String,
        #[serde(default)]
        expected_parsed_program: Option<String>,
        #[serde(default)]
        required_parsed_attributes: Vec<String>,
    },
    MultipleAccounts {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        required_value_attributes: Vec<String>,
        expected_value_attributes: serde_json::Value,
        expected_data_encoding: String,
        #[serde(default)]
        expected_parsed_program: Option<String>,
        #[serde(default)]
        required_parsed_attributes: Vec<String>,
    },
    ProgramAccounts {
        minimum_result_count: usize,
        required_result_attributes: Vec<String>,
        required_account_attributes: Vec<String>,
        expected_owner: String,
        expected_data_encoding: String,
        #[serde(default)]
        expected_parsed_program: Option<String>,
        #[serde(default)]
        required_parsed_attributes: Vec<String>,
    },
    RecentPerformanceSamples {
        minimum_result_count: usize,
        required_sample_attributes: Vec<String>,
    },
    RecentPrioritizationFees {
        minimum_result_count: usize,
        required_fee_attributes: Vec<String>,
    },
    SignaturesForAddress {
        minimum_result_count: usize,
        required_signature_attributes: Vec<String>,
    },
    SignatureStatuses {
        required_result_attributes: Vec<String>,
        required_context_attributes: Vec<String>,
        expected_value: serde_json::Value,
        expected_api_version: String,
    },
    TransactionSnapshot {
        required_result_attributes: Vec<String>,
        expected_result: serde_json::Value,
    },
    BlockSnapshot {
        required_result_attributes: Vec<String>,
        expected_result: serde_json::Value,
    },
}

fn default_required_response_attributes() -> Vec<String> {
    vec![
        "jsonrpc".to_string(),
        "result".to_string(),
        "id".to_string(),
    ]
}

pub fn load_rpc_fixtures(dir: impl AsRef<Path>) -> Result<Vec<RpcFixture>> {
    let mut fixtures = Vec::new();
    read_rpc_fixtures_recursive(dir.as_ref(), &mut fixtures)?;
    fixtures.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fixtures)
}

fn read_rpc_fixtures_recursive(dir: &Path, fixtures: &mut Vec<RpcFixture>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read fixture directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            read_rpc_fixtures_recursive(&path, fixtures)?;
            continue;
        }

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let fixture: RpcFixture = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse fixture {}", path.display()))?;
        fixtures.push(fixture);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rpc_fixture_with_generalized_schema() {
        let fixture: RpcFixture = serde_json::from_str(
            r#"{
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
            }"#,
        )
        .expect("fixture should parse");

        assert_eq!(fixture.method, "getHealth");
        assert!(fixture.request.params.is_empty());
        assert!(fixture.request.dynamic_params.is_empty());
        assert_eq!(
            fixture.expectation.envelope.required_attributes,
            vec!["jsonrpc", "result", "id"]
        );
        assert!(fixture.expectation.envelope.expected_error.is_none());
        match fixture.expectation.validator {
            MethodExpectation::StringResult { allowed_values } => {
                assert_eq!(allowed_values, vec!["ok"]);
            }
            MethodExpectation::BlockCommitment {
                required_result_attributes: _,
                expected_commitment: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::BlockTime { expected_value: _ } => {
                panic!("expected stringResult validator")
            }
            MethodExpectation::BlocksSnapshot { expected_result: _ } => {
                panic!("expected stringResult validator")
            }
            MethodExpectation::BlocksWithLimitSnapshot { expected_result: _ } => {
                panic!("expected stringResult validator")
            }
            MethodExpectation::BlockProduction {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
                required_range_attributes: _,
                expected_identity: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::ClusterNodes {
                minimum_result_count: _,
                required_node_attributes: _,
                required_string_attributes: _,
                nullable_string_attributes: _,
                required_u64_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::BlockHeight => panic!("expected stringResult validator"),
            MethodExpectation::EpochInfo {
                required_result_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::EpochSchedule {
                required_result_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::FeeForMessage {
                required_result_attributes: _,
                required_context_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::FirstAvailableBlock { expected_value: _ } => {
                panic!("expected stringResult validator")
            }
            MethodExpectation::GenesisHash => panic!("expected stringResult validator"),
            MethodExpectation::Identity {
                required_result_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::InflationGovernor {
                required_result_attributes: _,
                expected_result: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::InflationRate {
                required_result_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::InflationReward {
                expected_result_length: _,
                required_reward_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::LargestAccounts {
                minimum_result_count: _,
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::LeaderSchedule {
                minimum_validator_count: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::HighestSnapshotSlot {
                required_result_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::LatestBlockhash {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::Slot => panic!("expected stringResult validator"),
            MethodExpectation::SlotLeader => panic!("expected stringResult validator"),
            MethodExpectation::SlotLeaders {
                expected_result_length: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::StakeMinimumDelegation {
                required_result_attributes: _,
                required_context_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::Supply {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::TokenAccountBalance {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::MaxRetransmitSlot => panic!("expected stringResult validator"),
            MethodExpectation::MaxShredInsertSlot => panic!("expected stringResult validator"),
            MethodExpectation::MinimumBalanceForRentExemption { expected_value: _ } => {
                panic!("expected stringResult validator")
            }
            MethodExpectation::Balance {
                required_result_attributes: _,
                required_context_attributes: _,
                expected_value: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::AccountInfo {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
                expected_value_attributes: _,
                expected_owner: _,
                expected_data_encoding: _,
                expected_parsed_program: _,
                required_parsed_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::MultipleAccounts {
                required_result_attributes: _,
                required_context_attributes: _,
                required_value_attributes: _,
                expected_value_attributes: _,
                expected_data_encoding: _,
                expected_parsed_program: _,
                required_parsed_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::ProgramAccounts {
                minimum_result_count: _,
                required_result_attributes: _,
                required_account_attributes: _,
                expected_owner: _,
                expected_data_encoding: _,
                expected_parsed_program: _,
                required_parsed_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::RecentPerformanceSamples {
                minimum_result_count: _,
                required_sample_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::RecentPrioritizationFees {
                minimum_result_count: _,
                required_fee_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::SignaturesForAddress {
                minimum_result_count: _,
                required_signature_attributes: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::SignatureStatuses {
                required_result_attributes: _,
                required_context_attributes: _,
                expected_value: _,
                expected_api_version: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::TransactionSnapshot {
                required_result_attributes: _,
                expected_result: _,
            } => panic!("expected stringResult validator"),
            MethodExpectation::BlockSnapshot {
                required_result_attributes: _,
                expected_result: _,
            } => panic!("expected stringResult validator"),
        }
    }

    #[test]
    fn defaults_request_params_to_empty_array() {
        let fixture: RpcFixture = serde_json::from_str(
            r#"{
                "name": "default params",
                "method": "getHealth",
                "request": {},
                "expectation": {
                    "transport": {
                        "content_type_prefix": "application/json",
                        "charset": "utf-8"
                    },
                    "envelope": {
                        "jsonrpc_version": "2.0"
                    },
                    "validator": {
                        "kind": "stringResult",
                        "allowed_values": ["ok"]
                    }
                }
            }"#,
        )
        .expect("fixture should parse");

        assert!(fixture.request.params.is_empty());
        assert!(fixture.request.dynamic_params.is_empty());
        assert_eq!(
            fixture.expectation.envelope.required_attributes,
            vec!["jsonrpc", "result", "id"]
        );
        assert!(fixture.expectation.envelope.expected_error.is_none());
    }

    #[test]
    fn parses_dynamic_processed_slot_request_param() {
        let fixture: RpcFixture = serde_json::from_str(
            r#"{
                "name": "getSlotLeaders dynamic",
                "method": "getSlotLeaders",
                "request": {
                    "params": [null, 8],
                    "dynamic_params": [
                        {
                            "kind": "processedSlot",
                            "index": 0
                        }
                    ]
                },
                "expectation": {
                    "transport": {
                        "content_type_prefix": "application/json",
                        "charset": "utf-8"
                    },
                    "envelope": {
                        "jsonrpc_version": "2.0"
                    },
                    "validator": {
                        "kind": "slotLeaders",
                        "expected_result_length": 8
                    }
                }
            }"#,
        )
        .expect("fixture should parse");

        assert_eq!(
            fixture.request.params,
            vec![serde_json::json!(null), serde_json::json!(8)]
        );
        assert_eq!(fixture.request.dynamic_params.len(), 1);
        match &fixture.request.dynamic_params[0] {
            DynamicRequestParam::ProcessedSlot { index } => assert_eq!(*index, 0),
        }
    }
}
