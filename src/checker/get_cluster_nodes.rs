use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        minimum_result_count,
        required_node_attributes,
        required_string_attributes,
        nullable_string_attributes,
        required_u64_attributes,
    ) = match expectation {
        MethodExpectation::ClusterNodes {
            minimum_result_count,
            required_node_attributes,
            required_string_attributes,
            nullable_string_attributes,
            required_u64_attributes,
        } => (
            *minimum_result_count,
            required_node_attributes,
            required_string_attributes,
            nullable_string_attributes,
            required_u64_attributes,
        ),
        other => {
            anyhow::bail!("getClusterNodes expected a clusterNodes validator, received {other:?}")
        }
    };

    let result_array = result
        .as_array()
        .context("result field was not an array as required by the getClusterNodes validator")?;

    if result_array.len() < minimum_result_count {
        anyhow::bail!(
            "result array must contain at least {} element(s), received {}",
            minimum_result_count,
            result_array.len()
        );
    }

    let first_node = result_array
        .first()
        .and_then(Value::as_object)
        .context("result[0] was not an object")?;

    assert_required_attributes(first_node, required_node_attributes, "result[0]")?;

    for field_name in required_string_attributes {
        first_node
            .get(field_name)
            .and_then(Value::as_str)
            .with_context(|| format!("result[0].{field_name} was not a string"))?;
    }

    for field_name in nullable_string_attributes {
        let value = first_node
            .get(field_name)
            .with_context(|| format!("result[0] was missing required '{field_name}' field"))?;
        if !value.is_null() && value.as_str().is_none() {
            anyhow::bail!("result[0].{field_name} was neither null nor a string");
        }
    }

    for field_name in required_u64_attributes {
        first_node
            .get(field_name)
            .and_then(Value::as_u64)
            .with_context(|| format!("result[0].{field_name} was not a u64"))?;
    }

    let pubkey = first_node
        .get("pubkey")
        .and_then(Value::as_str)
        .context("result[0].pubkey was not a string")?;

    Ok(format!(
        "nodes={} firstPubkey={}",
        result_array.len(),
        pubkey
    ))
}

fn assert_required_attributes(
    object: &serde_json::Map<String, Value>,
    required_attributes: &[String],
    location: &str,
) -> Result<()> {
    for field_name in required_attributes {
        if !object.contains_key(field_name) {
            anyhow::bail!("{location} was missing required '{field_name}' field");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::ClusterNodes {
            minimum_result_count: 1,
            required_node_attributes: vec![
                "featureSet".to_string(),
                "gossip".to_string(),
                "pubkey".to_string(),
                "pubsub".to_string(),
                "rpc".to_string(),
                "serveRepair".to_string(),
                "shredVersion".to_string(),
                "tpu".to_string(),
                "tpuForwards".to_string(),
                "tpuForwardsQuic".to_string(),
                "tpuQuic".to_string(),
                "tpuVote".to_string(),
                "tvu".to_string(),
                "version".to_string(),
            ],
            required_string_attributes: vec![
                "gossip".to_string(),
                "pubkey".to_string(),
                "serveRepair".to_string(),
                "tpu".to_string(),
                "tpuForwards".to_string(),
                "tpuForwardsQuic".to_string(),
                "tpuQuic".to_string(),
                "tpuVote".to_string(),
                "tvu".to_string(),
                "version".to_string(),
            ],
            nullable_string_attributes: vec!["pubsub".to_string(), "rpc".to_string()],
            required_u64_attributes: vec!["featureSet".to_string(), "shredVersion".to_string()],
        }
    }

    #[test]
    fn validates_cluster_nodes_shape() {
        let result = validate(
            &expectation(),
            &json!([
                {
                    "featureSet": 1,
                    "gossip": "10.0.0.1:8001",
                    "pubkey": "node-1",
                    "pubsub": null,
                    "rpc": "10.0.0.1:8899",
                    "serveRepair": "10.0.0.1:8002",
                    "shredVersion": 2,
                    "tpu": "10.0.0.1:8856",
                    "tpuForwards": "10.0.0.1:8857",
                    "tpuForwardsQuic": "10.0.0.1:8863",
                    "tpuQuic": "10.0.0.1:8862",
                    "tpuVote": "10.0.0.1:8858",
                    "tvu": "10.0.0.1:8000",
                    "version": "3.1.11"
                }
            ]),
        )
        .expect("expected success");

        assert_eq!(result, "nodes=1 firstPubkey=node-1");
    }

    #[test]
    fn rejects_empty_result_array() {
        let error = validate(&expectation(), &json!([])).expect_err("empty array should fail");

        assert!(
            error
                .to_string()
                .contains("result array must contain at least 1 element")
        );
    }
}
