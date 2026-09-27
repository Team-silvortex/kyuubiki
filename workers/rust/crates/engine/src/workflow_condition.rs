use serde_json::{Map, Number, Value};
use std::cmp::Ordering;

pub(super) fn evaluate_condition_operator(payload: &Value, config: &Value) -> Result<bool, String> {
    let empty = Map::new();
    let predicate = match config {
        Value::Null => &empty,
        Value::Object(config) => match config.get("predicate") {
            None => &empty,
            Some(Value::Object(predicate)) => predicate,
            _ => return Err("condition config.predicate must be an object".into()),
        },
        _ => return Err("condition config must be an object or null".into()),
    };
    let operator = match predicate.get("operator") {
        None => "gt",
        Some(Value::String(value))
            if matches!(
                value.as_str(),
                "truthy" | "falsy" | "eq" | "neq" | "gt" | "gte" | "lt" | "lte" | "contains"
            ) =>
        {
            value.as_str()
        }
        Some(Value::String(_)) => {
            return Err("unsupported condition operator at config.predicate.operator".into());
        }
        _ => {
            return Err(
                "condition config.predicate.operator must name a supported operator".into(),
            );
        }
    };
    let target = match predicate.get("path") {
        None | Some(Value::Null) => payload,
        Some(Value::String(path)) => resolve_target(payload, path),
        _ => return Err("condition config.predicate.path must be a string or null".into()),
    };
    let value = predicate.get("value").unwrap_or(&Value::Null);

    match operator {
        "truthy" => Ok(is_truthy(target)),
        "falsy" => Ok(!is_truthy(target)),
        "eq" => Ok(json_equal(target, value)),
        "neq" => Ok(!json_equal(target, value)),
        "contains" => match target {
            Value::String(text) => value
                .as_str()
                .map(|needle| text.contains(needle))
                .ok_or_else(|| {
                    "condition config.predicate.value must be a string for string contains".into()
                }),
            Value::Array(items) => Ok(items.iter().any(|item| json_equal(item, value))),
            _ => Err("condition operator contains expects string or array input".into()),
        },
        _ => {
            let left = target
                .as_number()
                .ok_or("condition operator expects numeric input")?;
            let right = value
                .as_number()
                .ok_or("condition config.predicate.value must be numeric")?;
            let order =
                number_cmp(left, right).ok_or("condition operands must be finite numbers")?;
            Ok(match operator {
                "gt" => order.is_gt(),
                "gte" => !order.is_lt(),
                "lt" => order.is_lt(),
                "lte" => !order.is_gt(),
                _ => unreachable!("operator was validated"),
            })
        }
    }
}

fn resolve_target<'a>(payload: &'a Value, path: &str) -> &'a Value {
    let mut current = payload;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        current = match current {
            Value::Object(map) => map.get(segment).unwrap_or(&Value::Null),
            Value::Array(items) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get(index))
                .unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    current
}

fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => number_cmp(a, b) == Some(Ordering::Equal),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| json_equal(a, b)))
        }
        _ => left == right,
    }
}

fn integer(number: &Number) -> Option<(bool, u64)> {
    number
        .as_i64()
        .map(|value| (value < 0, value.unsigned_abs()))
        .or_else(|| number.as_u64().map(|value| (false, value)))
}

fn number_cmp(left: &Number, right: &Number) -> Option<Ordering> {
    match (integer(left), integer(right)) {
        (Some((an, a)), Some((bn, b))) => Some(if an != bn {
            bn.cmp(&an)
        } else if an {
            b.cmp(&a)
        } else {
            a.cmp(&b)
        }),
        (Some(a), None) => right
            .as_f64()
            .filter(|b| b.is_finite())
            .map(|b| integer_float_cmp(a, b)),
        (None, Some(b)) => left
            .as_f64()
            .filter(|a| a.is_finite())
            .map(|a| integer_float_cmp(b, a).reverse()),
        (None, None) => left.as_f64()?.partial_cmp(&right.as_f64()?),
    }
}

fn integer_float_cmp((negative, magnitude): (bool, u64), value: f64) -> Ordering {
    let float_negative = value < 0.0;
    if negative != float_negative {
        return float_negative.cmp(&negative);
    }
    let absolute = value.abs();
    // Compare the integer part exactly; casting the integer to f64 loses bits above 2^53.
    let order = if absolute >= 18_446_744_073_709_551_616.0 {
        Ordering::Less
    } else {
        match magnitude.cmp(&(absolute as u64)) {
            Ordering::Equal if absolute.fract() != 0.0 => Ordering::Less,
            order => order,
        }
    };
    if negative { order.reverse() } else { order }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|entry| entry != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_order_matches_i128_oracle_without_float_rounding() {
        let values: Vec<i128> = vec![
            i64::MIN.into(),
            -9_007_199_254_740_993,
            -1,
            0,
            1,
            9_007_199_254_740_992,
            9_007_199_254_740_993,
            i64::MAX.into(),
            u64::MAX as i128 - 1,
            u64::MAX.into(),
        ];
        let number = |value: i128| {
            if value < 0 {
                Number::from(value as i64)
            } else {
                Number::from(value as u64)
            }
        };
        for a in &values {
            for b in &values {
                assert_eq!(
                    number_cmp(&number(*a), &number(*b)),
                    Some(a.cmp(b)),
                    "{a} vs {b}"
                );
            }
        }
    }

    #[test]
    fn mixed_order_matches_exact_quarter_oracle_in_both_directions() {
        for integer in -32_i64..=32 {
            for quarter in -128_i64..=128 {
                let a = Number::from(integer);
                let b = Number::from_f64(quarter as f64 / 4.0).unwrap();
                let expected = (integer * 4).cmp(&quarter);
                assert_eq!(number_cmp(&a, &b), Some(expected));
                assert_eq!(number_cmp(&b, &a), Some(expected.reverse()));
            }
        }
    }
}
