#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RequestParts {
    pub(crate) query: Vec<(String, String)>,
    pub(crate) json_body: Option<serde_json::Value>,
}

#[allow(dead_code)]
impl RequestParts {
    pub(crate) fn with_query(query: Vec<(String, String)>) -> Self {
        Self {
            query,
            json_body: None,
        }
    }

    pub(crate) fn with_json(json_body: serde_json::Value) -> Self {
        Self {
            query: Vec::new(),
            json_body: Some(json_body),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoContent;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{NoContent, RequestParts};

    #[test]
    fn with_query_keeps_query_and_omits_json_body() {
        let parts = RequestParts::with_query(vec![("symbols".to_owned(), "AAPL,MSFT".to_owned())]);

        assert_eq!(
            parts,
            RequestParts {
                query: vec![("symbols".to_owned(), "AAPL,MSFT".to_owned())],
                json_body: None,
            }
        );
    }

    #[test]
    fn with_json_keeps_json_body_and_clears_query() {
        let parts = RequestParts::with_json(json!({ "symbol": "AAPL" }));

        assert_eq!(parts.query, Vec::<(String, String)>::new());
        assert_eq!(parts.json_body, Some(json!({ "symbol": "AAPL" })));
    }

    #[test]
    fn no_content_is_constructible() {
        assert_eq!(NoContent, NoContent::default());
    }
}
