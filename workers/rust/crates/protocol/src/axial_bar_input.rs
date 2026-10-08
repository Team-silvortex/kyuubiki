use crate::SolveBarRequest;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

const MAX_PORTABLE_ELEMENTS: f64 = u32::MAX as f64;
const UNIT_ROUNDOFF: f64 = 4.0 * f64::EPSILON;

// This compatibility input is a protocol adapter, not an engine request type.
// Unknown metadata is skipped by the derived reader rather than buffered as Value.
#[derive(Deserialize)]
pub(crate) struct AxialBarInput {
    length: Number,
    area: Number,
    elements: Number,
    tip_force: Number,
    #[serde(default)]
    youngs_modulus: OptionalNumber,
    #[serde(default)]
    youngs_modulus_gpa: OptionalNumber,
}

impl TryFrom<AxialBarInput> for SolveBarRequest {
    type Error = &'static str;

    fn try_from(input: AxialBarInput) -> Result<Self, Self::Error> {
        let pa = input.youngs_modulus.0;
        let gpa = input.youngs_modulus_gpa.0;
        let scaled = gpa.map(|value| value * 1.0e9);
        if pa.is_some_and(|value| value <= 0.0)
            || scaled.is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("axial bar modulus must be finite and positive");
        }
        if let (Some(pa), Some(scaled)) = (pa, scaled)
            && (pa - scaled).abs() > UNIT_ROUNDOFF * pa.abs().max(scaled.abs())
        {
            return Err("axial bar Pa and GPa moduli contradict each other");
        }
        let youngs_modulus = scaled
            .or(pa)
            .ok_or("axial bar requires a Pa or GPa modulus")?;
        if input.length.0 <= 0.0 || input.area.0 <= 0.0 {
            return Err("axial bar length and area must be positive");
        }
        let elements = input.elements.0;
        if !(1.0..=MAX_PORTABLE_ELEMENTS).contains(&elements) || elements.fract() != 0.0 {
            return Err("axial bar elements must be an integer in 1..=4294967295");
        }
        Ok(Self {
            length: input.length.0,
            area: input.area.0,
            youngs_modulus,
            elements: elements as usize,
            tip_force: input.tip_force.0,
        })
    }
}

struct Number(f64);

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Numeric;
        impl Visitor<'_> for Numeric {
            type Value = Number;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a finite number or strict JSON-number string")
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Number, E> {
                if value.is_finite() {
                    Ok(Number(value))
                } else {
                    Err(E::custom("axial bar numbers must be finite"))
                }
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Number, E> {
                self.visit_f64(value as f64)
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Number, E> {
                self.visit_f64(value as f64)
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Number, E> {
                if value.is_empty() || value.len() > 256 || value.trim() != value {
                    return Err(E::custom("invalid axial bar numeric string"));
                }
                let number = serde_json::from_str::<f64>(value)
                    .map_err(|_| E::custom("invalid axial bar numeric string"))?;
                self.visit_f64(number)
            }
        }
        deserializer.deserialize_any(Numeric)
    }
}

#[derive(Default)]
struct OptionalNumber(Option<f64>);

impl<'de> Deserialize<'de> for OptionalNumber {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Explicit null is invalid; only an absent field uses Default.
        Number::deserialize(deserializer).map(|number| Self(Some(number.0)))
    }
}
