#![allow(dead_code)]

use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(serde_json::Number),
}

fn parse_decimal<E>(value: StringOrNumber) -> Result<Decimal, E>
where
    E: de::Error,
{
    let raw = match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    };

    Decimal::from_str(&raw)
        .map_err(|error| E::custom(format!("invalid decimal value `{raw}`: {error}")))
}

pub(crate) fn deserialize_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    parse_decimal(StringOrNumber::deserialize(deserializer)?)
}

pub(crate) fn deserialize_option_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrNumber>::deserialize(deserializer)?
        .map(parse_decimal)
        .transpose()
}

pub(crate) mod string_contract {
    use super::*;

    pub(crate) fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub(crate) fn serialize_option_decimal<S>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_decimal(value, serializer),
            None => serializer.serialize_none(),
        }
    }
}

pub(crate) mod number_contract {
    use super::*;

    pub(crate) fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_json::Value::Number(decimal_to_json_number::<S>(value)?).serialize(serializer)
    }

    pub(crate) fn serialize_option_decimal<S>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_decimal(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    fn decimal_to_json_number<S>(value: &Decimal) -> Result<serde_json::Number, S::Error>
    where
        S: Serializer,
    {
        serde_json::Number::from_str(&value.to_string()).map_err(|error| {
            serde::ser::Error::custom(format!(
                "decimal cannot be serialized as JSON number: {error}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        deserialize_decimal_from_string_or_number,
        deserialize_option_decimal_from_string_or_number, number_contract, string_contract,
    };
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct DecimalField {
        #[serde(deserialize_with = "deserialize_decimal_from_string_or_number")]
        value: Decimal,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct OptionalDecimalField {
        #[serde(
            default,
            deserialize_with = "deserialize_option_decimal_from_string_or_number"
        )]
        value: Option<Decimal>,
    }

    #[derive(Debug, Serialize)]
    struct StringDecimalField {
        #[serde(serialize_with = "string_contract::serialize_decimal")]
        value: Decimal,
    }

    #[derive(Debug, Serialize)]
    struct OptionalStringDecimalField {
        #[serde(serialize_with = "string_contract::serialize_option_decimal")]
        value: Option<Decimal>,
    }

    #[derive(Debug, Serialize)]
    struct NumberDecimalField {
        #[serde(serialize_with = "number_contract::serialize_decimal")]
        value: Decimal,
    }

    #[derive(Debug, Serialize)]
    struct OptionalNumberDecimalField {
        #[serde(serialize_with = "number_contract::serialize_option_decimal")]
        value: Option<Decimal>,
    }

    #[test]
    fn decimal_from_string_or_number_deserializes_numeric_string() {
        let payload = r#"{"value":"123.45"}"#;

        let decoded: DecimalField =
            serde_json::from_str(payload).expect("numeric string should deserialize");

        assert_eq!(decoded.value, Decimal::new(12345, 2));
    }

    #[test]
    fn decimal_from_string_or_number_deserializes_json_number() {
        let payload = r#"{"value":123.45}"#;

        let decoded: DecimalField =
            serde_json::from_str(payload).expect("json number should deserialize");

        assert_eq!(decoded.value, Decimal::new(12345, 2));
    }

    #[test]
    fn option_decimal_from_string_or_number_handles_some_and_none() {
        let some_payload = r#"{"value":"123.45"}"#;
        let none_payload = r#"{"value":null}"#;
        let missing_payload = r#"{}"#;

        let some: OptionalDecimalField =
            serde_json::from_str(some_payload).expect("numeric string should deserialize");
        let none: OptionalDecimalField =
            serde_json::from_str(none_payload).expect("null should deserialize");
        let missing: OptionalDecimalField =
            serde_json::from_str(missing_payload).expect("missing field should deserialize");

        assert_eq!(some.value, Some(Decimal::new(12345, 2)));
        assert_eq!(none.value, None);
        assert_eq!(missing.value, None);
    }

    #[test]
    fn string_contract_serializes_decimal_and_option_as_json_strings() {
        let value = StringDecimalField {
            value: Decimal::new(12345, 2),
        };
        let optional_some = OptionalStringDecimalField {
            value: Some(Decimal::new(6789, 2)),
        };
        let optional_none = OptionalStringDecimalField { value: None };

        assert_eq!(
            serde_json::to_string(&value).expect("string decimal should serialize"),
            r#"{"value":"123.45"}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_some)
                .expect("optional string decimal should serialize"),
            r#"{"value":"67.89"}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_none).expect("optional none should serialize"),
            r#"{"value":null}"#
        );
    }

    #[test]
    fn number_contract_serializes_decimal_and_option_as_json_numbers() {
        let value = NumberDecimalField {
            value: Decimal::new(12345, 2),
        };
        let optional_some = OptionalNumberDecimalField {
            value: Some(Decimal::new(6789, 2)),
        };
        let optional_none = OptionalNumberDecimalField { value: None };

        assert_eq!(
            serde_json::to_string(&value).expect("number decimal should serialize"),
            r#"{"value":123.45}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_some)
                .expect("optional number decimal should serialize"),
            r#"{"value":67.89}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_none).expect("optional none should serialize"),
            r#"{"value":null}"#
        );
    }
}
