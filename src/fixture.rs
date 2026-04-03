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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MethodExpectation {
    StringResult {
        allowed_values: Vec<String>,
    },
    EpochInfo {
        required_result_attributes: Vec<String>,
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
        assert_eq!(
            fixture.expectation.envelope.required_attributes,
            vec!["jsonrpc", "result", "id"]
        );
        match fixture.expectation.validator {
            MethodExpectation::StringResult { allowed_values } => {
                assert_eq!(allowed_values, vec!["ok"]);
            }
            MethodExpectation::EpochInfo {
                required_result_attributes: _,
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
        assert_eq!(
            fixture.expectation.envelope.required_attributes,
            vec!["jsonrpc", "result", "id"]
        );
    }
}
