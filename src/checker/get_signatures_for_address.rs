use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (minimum_result_count, required_signature_attributes) = match expectation {
        MethodExpectation::SignaturesForAddress {
            minimum_result_count,
            required_signature_attributes,
        } => (*minimum_result_count, required_signature_attributes),
        other => anyhow::bail!(
            "getSignaturesForAddress expected a signaturesForAddress validator, received {other:?}"
        ),
    };

    let entries = result.as_array().context(
        "result field was not an array as required by the getSignaturesForAddress validator",
    )?;

    if entries.len() < minimum_result_count {
        anyhow::bail!(
            "result array length {} was smaller than the required minimum {}",
            entries.len(),
            minimum_result_count
        );
    }

    for (index, entry) in entries.iter().enumerate() {
        let entry_object = entry
            .as_object()
            .with_context(|| format!("result[{index}] was not an object"))?;
        assert_required_attributes(
            entry_object,
            required_signature_attributes,
            &format!("result[{index}]"),
        )?;

        entry_object
            .get("signature")
            .and_then(Value::as_str)
            .with_context(|| format!("result[{index}].signature was not a string"))?;
        entry_object
            .get("slot")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].slot was not a u64"))?;

        let block_time = entry_object
            .get("blockTime")
            .with_context(|| format!("result[{index}] was missing required 'blockTime' field"))?;
        if !block_time.is_null() && block_time.as_i64().is_none() {
            anyhow::bail!("result[{index}].blockTime was neither null nor an i64");
        }

        let memo = entry_object
            .get("memo")
            .with_context(|| format!("result[{index}] was missing required 'memo' field"))?;
        if !memo.is_null() && memo.as_str().is_none() {
            anyhow::bail!("result[{index}].memo was neither null nor a string");
        }

        let confirmation_status = entry_object.get("confirmationStatus").with_context(|| {
            format!("result[{index}] was missing required 'confirmationStatus' field")
        })?;
        if !confirmation_status.is_null() && confirmation_status.as_str().is_none() {
            anyhow::bail!("result[{index}].confirmationStatus was neither null nor a string");
        }

        let err = entry_object
            .get("err")
            .with_context(|| format!("result[{index}] was missing required 'err' field"))?;
        if !err.is_null() && !err.is_object() {
            anyhow::bail!("result[{index}].err was neither null nor an object");
        }
    }

    Ok(format!("signatures={}", entries.len()))
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
        MethodExpectation::SignaturesForAddress {
            minimum_result_count: 1,
            required_signature_attributes: vec![
                "blockTime".to_string(),
                "confirmationStatus".to_string(),
                "err".to_string(),
                "memo".to_string(),
                "signature".to_string(),
                "slot".to_string(),
            ],
        }
    }

    #[test]
    fn validates_signatures_for_address_shape() {
        let result = validate(
            &expectation(),
            &json!([
                {
                    "blockTime": 1_775_432_481i64,
                    "confirmationStatus": "finalized",
                    "err": null,
                    "memo": null,
                    "signature": "abc",
                    "slot": 123
                }
            ]),
        )
        .expect("expected success");

        assert_eq!(result, "signatures=1");
    }

    #[test]
    fn rejects_missing_signature_field() {
        let error = validate(
            &expectation(),
            &json!([
                {
                    "blockTime": 1_775_432_481i64,
                    "confirmationStatus": "finalized",
                    "err": null,
                    "memo": null,
                    "slot": 123
                }
            ]),
        )
        .expect_err("missing signature should fail");

        assert!(
            error
                .to_string()
                .contains("result[0] was missing required 'signature' field")
        );
    }
}
