use crate::common::query::QueryWriter;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub start: Option<String>,
    pub end: Option<String>,
}

impl ListRequest {
    pub(crate) fn to_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::ListRequest;

    #[test]
    fn list_request_serializes_official_query_words() {
        let query = ListRequest {
            start: Some("2026-04-01".to_owned()),
            end: Some("2026-04-30".to_owned()),
        }
        .to_query();

        assert_eq!(
            query,
            vec![
                ("start".to_owned(), "2026-04-01".to_owned()),
                ("end".to_owned(), "2026-04-30".to_owned()),
            ]
        );
    }

    #[test]
    fn list_request_omits_none_fields() {
        let query = ListRequest::default().to_query();

        assert!(query.is_empty());
    }
}
