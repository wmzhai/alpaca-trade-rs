use serde::{Deserialize, Deserializer, Serializer, de};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(serde_json::Number),
}

fn parse_u32<E>(value: StringOrNumber) -> Result<u32, E>
where
    E: de::Error,
{
    let raw = match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    };

    raw.parse::<u32>()
        .map_err(|error| E::custom(format!("invalid integer value `{raw}`: {error}")))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn deserialize_u32_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    parse_u32(StringOrNumber::deserialize(deserializer)?)
}

pub(crate) fn deserialize_option_u32_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrNumber>::deserialize(deserializer)?
        .map(parse_u32)
        .transpose()
}

pub(crate) mod string_contract {
    use super::*;

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn serialize_u32<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub(crate) fn serialize_option_u32<S>(
        value: &Option<u32>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_u32(value, serializer),
            None => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        deserialize_option_u32_from_string_or_number, deserialize_u32_from_string_or_number,
        string_contract,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct IntegerField {
        #[serde(deserialize_with = "deserialize_u32_from_string_or_number")]
        value: u32,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct OptionalIntegerField {
        #[serde(
            default,
            deserialize_with = "deserialize_option_u32_from_string_or_number"
        )]
        value: Option<u32>,
    }

    #[derive(Debug, Serialize)]
    struct StringIntegerField {
        #[serde(serialize_with = "string_contract::serialize_u32")]
        value: u32,
    }

    #[derive(Debug, Serialize)]
    struct OptionalStringIntegerField {
        #[serde(serialize_with = "string_contract::serialize_option_u32")]
        value: Option<u32>,
    }

    #[test]
    fn u32_from_string_or_number_deserializes_string_and_number() {
        let from_string: IntegerField =
            serde_json::from_str(r#"{"value":"2"}"#).expect("string integer should deserialize");
        let from_number: IntegerField =
            serde_json::from_str(r#"{"value":2}"#).expect("number integer should deserialize");

        assert_eq!(from_string.value, 2);
        assert_eq!(from_number.value, 2);
    }

    #[test]
    fn option_u32_from_string_or_number_handles_some_and_none() {
        let some: OptionalIntegerField =
            serde_json::from_str(r#"{"value":"3"}"#).expect("string integer should deserialize");
        let none: OptionalIntegerField =
            serde_json::from_str(r#"{"value":null}"#).expect("null should deserialize");
        let missing: OptionalIntegerField =
            serde_json::from_str(r#"{}"#).expect("missing field should deserialize");

        assert_eq!(some.value, Some(3));
        assert_eq!(none.value, None);
        assert_eq!(missing.value, None);
    }

    #[test]
    fn u32_string_contract_serializes_integer_and_option() {
        let value = StringIntegerField { value: 2 };
        let optional_some = OptionalStringIntegerField { value: Some(3) };
        let optional_none = OptionalStringIntegerField { value: None };

        assert_eq!(
            serde_json::to_string(&value).expect("integer should serialize"),
            r#"{"value":"2"}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_some).expect("optional integer should serialize"),
            r#"{"value":"3"}"#
        );
        assert_eq!(
            serde_json::to_string(&optional_none).expect("optional none should serialize"),
            r#"{"value":null}"#
        );
    }

    #[test]
    fn u32_from_string_or_number_rejects_decimal_values() {
        let error =
            serde_json::from_str::<IntegerField>(r#"{"value":"1.5"}"#).expect_err("must fail");

        assert!(error.to_string().contains("invalid integer value `1.5`"));
    }
}
