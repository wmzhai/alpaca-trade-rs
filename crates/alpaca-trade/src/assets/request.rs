use crate::common::query::QueryWriter;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub status: Option<String>,
    pub asset_class: Option<String>,
    pub exchange: Option<String>,
    pub attributes: Option<Vec<String>>,
}

impl ListRequest {
    #[allow(dead_code)]
    pub(crate) fn to_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("status", self.status);
        query.push_opt("asset_class", self.asset_class);
        query.push_opt("exchange", self.exchange);
        query.push_csv("attributes", self.attributes.unwrap_or_default());
        query.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::ListRequest;

    #[test]
    fn list_request_serializes_official_query_words() {
        let query = ListRequest {
            status: Some("active".to_owned()),
            asset_class: Some("us_equity".to_owned()),
            exchange: Some("NASDAQ".to_owned()),
            attributes: Some(vec![
                "ptp_no_exception".to_owned(),
                "has_options".to_owned(),
            ]),
        }
        .to_query();

        assert_eq!(
            query,
            vec![
                ("status".to_owned(), "active".to_owned()),
                ("asset_class".to_owned(), "us_equity".to_owned()),
                ("exchange".to_owned(), "NASDAQ".to_owned()),
                (
                    "attributes".to_owned(),
                    "ptp_no_exception,has_options".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn list_request_omits_none_and_empty_attributes() {
        let query = ListRequest::default().to_query();
        assert!(query.is_empty());

        let query = ListRequest {
            attributes: Some(Vec::new()),
            ..ListRequest::default()
        }
        .to_query();

        assert!(query.is_empty());
    }
}
