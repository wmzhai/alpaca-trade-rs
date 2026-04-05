use crate::error::Error;

#[allow(dead_code)]
pub(crate) fn required_path_segment(name: &'static str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!("{name} must not be blank")));
    }

    if trimmed.chars().any(|ch| matches!(ch, '/' | '?' | '#'))
        || contains_encoded_reserved_path_characters(trimmed)
    {
        return Err(Error::InvalidRequest(format!(
            "{name} must not contain reserved path characters"
        )));
    }

    Ok(trimmed.to_owned())
}

fn contains_encoded_reserved_path_characters(value: &str) -> bool {
    let bytes = value.as_bytes();

    bytes.windows(3).any(|window| {
        window[0] == b'%'
            && matches!(
                (window[1].to_ascii_lowercase(), window[2].to_ascii_lowercase()),
                (b'2', b'f') | (b'3', b'f') | (b'2', b'3')
            )
    })
}

#[allow(dead_code)]
pub(crate) fn validate_limit(limit: u32, max: u32) -> Result<u32, Error> {
    if limit == 0 {
        return Err(Error::InvalidRequest(
            "limit must be greater than 0".to_owned(),
        ));
    }

    if limit > max {
        return Err(Error::InvalidRequest(format!(
            "limit must be less than or equal to {max}"
        )));
    }

    Ok(limit)
}

#[cfg(test)]
mod tests {
    use super::{required_path_segment, validate_limit};
    use crate::error::Error;

    #[test]
    fn required_path_segment_rejects_blank_values() {
        let error = required_path_segment("symbol_or_asset_id", "   ")
            .expect_err("blank path segments should fail");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("symbol_or_asset_id"));
            }
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    #[test]
    fn required_path_segment_rejects_reserved_url_characters() {
        for value in [
            "AAPL/US",
            "AAPL?draft=true",
            "AAPL#fragment",
            "AAPL%2FUS",
            "AAPL%2fus",
            "AAPL%3Fdraft=true",
            "AAPL%23fragment",
        ] {
            let error = required_path_segment("symbol_or_asset_id", value)
                .expect_err("reserved URL characters should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("symbol_or_asset_id"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn validate_limit_rejects_zero() {
        let error = validate_limit(0, 500).expect_err("zero limit should fail");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("greater than 0"));
            }
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    #[test]
    fn validate_limit_rejects_values_above_max() {
        let error = validate_limit(501, 500).expect_err("out-of-range limit should fail");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("500"));
            }
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    #[test]
    fn validate_limit_accepts_in_range_values() {
        assert_eq!(
            validate_limit(200, 500).expect("limit should be accepted"),
            200
        );
    }
}
