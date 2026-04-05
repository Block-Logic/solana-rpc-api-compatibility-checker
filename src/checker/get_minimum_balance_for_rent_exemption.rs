use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_value = match expectation {
        MethodExpectation::MinimumBalanceForRentExemption { expected_value } => *expected_value,
        other => anyhow::bail!(
            "getMinimumBalanceForRentExemption expected a minimumBalanceForRentExemption validator, received {other:?}"
        ),
    };

    let value = result.as_u64().context(
        "result field was not a u64 as required by the getMinimumBalanceForRentExemption validator",
    )?;

    if value != expected_value {
        anyhow::bail!("result expected {}, received {}", expected_value, value);
    }

    Ok(format!("minimumBalanceForRentExemption={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_minimum_balance_for_rent_exemption() {
        let result = validate(
            &MethodExpectation::MinimumBalanceForRentExemption {
                expected_value: 1_586_880,
            },
            &json!(1_586_880u64),
        )
        .expect("expected success");

        assert_eq!(result, "minimumBalanceForRentExemption=1586880");
    }

    #[test]
    fn rejects_minimum_balance_for_rent_exemption_mismatch() {
        let error = validate(
            &MethodExpectation::MinimumBalanceForRentExemption {
                expected_value: 1_586_880,
            },
            &json!(1_586_881u64),
        )
        .expect_err("mismatched value should fail");

        assert!(
            error
                .to_string()
                .contains("result expected 1586880, received 1586881")
        );
    }
}
