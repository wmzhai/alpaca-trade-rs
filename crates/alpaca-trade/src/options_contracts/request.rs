use std::fmt;

use rust_decimal::Decimal;

use crate::common::pagination::PaginatedRequest;
use crate::common::query::QueryWriter;
use crate::common::validate::{required_text, validate_limit};
use crate::error::Error;

use super::{ContractStatus, ContractStyle, ContractType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub underlying_symbols: Option<Vec<String>>,
    pub show_deliverables: Option<bool>,
    pub status: Option<ContractStatus>,
    pub expiration_date: Option<String>,
    pub expiration_date_gte: Option<String>,
    pub expiration_date_lte: Option<String>,
    pub root_symbol: Option<String>,
    pub r#type: Option<ContractType>,
    pub style: Option<ContractStyle>,
    pub strike_price_gte: Option<Decimal>,
    pub strike_price_lte: Option<Decimal>,
    pub page_token: Option<String>,
    pub limit: Option<u32>,
    pub ppind: Option<bool>,
}

impl ListRequest {
    #[allow(dead_code)]
    pub(crate) fn to_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_csv(
            "underlying_symbols",
            validate_symbols(self.underlying_symbols)?,
        );
        query.push_opt("show_deliverables", self.show_deliverables);
        query.push_opt("status", self.status);
        query.push_opt(
            "expiration_date",
            validate_optional_text("expiration_date", self.expiration_date)?,
        );
        query.push_opt(
            "expiration_date_gte",
            validate_optional_text("expiration_date_gte", self.expiration_date_gte)?,
        );
        query.push_opt(
            "expiration_date_lte",
            validate_optional_text("expiration_date_lte", self.expiration_date_lte)?,
        );
        query.push_opt(
            "root_symbol",
            validate_optional_text("root_symbol", self.root_symbol)?,
        );
        query.push_opt("type", self.r#type);
        query.push_opt("style", self.style);
        query.push_opt("strike_price_gte", self.strike_price_gte);
        query.push_opt("strike_price_lte", self.strike_price_lte);
        query.push_opt(
            "page_token",
            validate_optional_text("page_token", self.page_token)?,
        );
        query.push_opt(
            "limit",
            self.limit
                .map(|limit| validate_limit(limit, 10_000))
                .transpose()?,
        );
        query.push_opt("ppind", self.ppind);
        Ok(query.finish())
    }
}

impl PaginatedRequest for ListRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

#[allow(dead_code)]
fn validate_optional_text(
    name: &'static str,
    value: Option<String>,
) -> Result<Option<String>, Error> {
    value.map(|value| required_text(name, &value)).transpose()
}

#[allow(dead_code)]
fn validate_symbols(value: Option<Vec<String>>) -> Result<Vec<String>, Error> {
    match value {
        None => Ok(Vec::new()),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(
            "underlying_symbols must contain at least one symbol".to_owned(),
        )),
        Some(values) => values
            .into_iter()
            .map(|value| required_text("underlying_symbols", &value))
            .collect(),
    }
}

impl fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractStatus::Active => "active",
            ContractStatus::Inactive => "inactive",
        })
    }
}

impl fmt::Display for ContractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractType::Call => "call",
            ContractType::Put => "put",
        })
    }
}

impl fmt::Display for ContractStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractStyle::American => "american",
            ContractStyle::European => "european",
        })
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::ListRequest;
    use crate::common::pagination::PaginatedRequest;
    use crate::error::Error;
    use crate::options_contracts::{ContractStatus, ContractStyle, ContractType};

    #[test]
    fn list_request_serializes_official_query_words() {
        let query = ListRequest {
            underlying_symbols: Some(vec!["AAPL".into(), "SPY".into()]),
            show_deliverables: Some(true),
            status: Some(ContractStatus::Active),
            expiration_date: Some("2026-06-19".into()),
            expiration_date_gte: Some("2026-06-01".into()),
            expiration_date_lte: Some("2026-06-30".into()),
            root_symbol: Some("AAPL".into()),
            r#type: Some(ContractType::Call),
            style: Some(ContractStyle::American),
            strike_price_gte: Some(Decimal::new(10_000, 2)),
            strike_price_lte: Some(Decimal::new(20_000, 2)),
            page_token: Some("MTAwMA==".into()),
            limit: Some(100),
            ppind: Some(true),
        }
        .to_query()
        .expect("query should serialize");

        assert_eq!(
            query,
            vec![
                ("underlying_symbols".to_owned(), "AAPL,SPY".to_owned()),
                ("show_deliverables".to_owned(), "true".to_owned()),
                ("status".to_owned(), "active".to_owned()),
                ("expiration_date".to_owned(), "2026-06-19".to_owned()),
                ("expiration_date_gte".to_owned(), "2026-06-01".to_owned()),
                ("expiration_date_lte".to_owned(), "2026-06-30".to_owned()),
                ("root_symbol".to_owned(), "AAPL".to_owned()),
                ("type".to_owned(), "call".to_owned()),
                ("style".to_owned(), "american".to_owned()),
                ("strike_price_gte".to_owned(), "100.00".to_owned()),
                ("strike_price_lte".to_owned(), "200.00".to_owned()),
                ("page_token".to_owned(), "MTAwMA==".to_owned()),
                ("limit".to_owned(), "100".to_owned()),
                ("ppind".to_owned(), "true".to_owned()),
            ]
        );
    }

    #[test]
    fn list_request_rejects_empty_underlying_symbols() {
        let error = ListRequest {
            underlying_symbols: Some(Vec::new()),
            ..ListRequest::default()
        }
        .to_query()
        .expect_err("empty symbols must fail");

        assert!(matches!(
            error,
            Error::InvalidRequest(message)
                if message.contains("underlying_symbols")
                    && message.contains("at least one symbol")
        ));
    }

    #[test]
    fn list_request_rejects_whitespace_padded_text_inputs() {
        let error = ListRequest {
            underlying_symbols: Some(vec![" AAPL ".into()]),
            page_token: Some(" token ".into()),
            ..ListRequest::default()
        }
        .to_query()
        .expect_err("whitespace-padded fields must fail");

        assert!(matches!(
            error,
            Error::InvalidRequest(message)
                if message.contains("leading or trailing whitespace")
        ));
    }

    #[test]
    fn list_request_with_page_token_replaces_existing_cursor_without_mutating_original() {
        let request = ListRequest {
            page_token: Some("cursor-1".into()),
            ..ListRequest::default()
        };

        let next = request.with_page_token(Some("cursor-2".into()));

        assert_eq!(request.page_token.as_deref(), Some("cursor-1"));
        assert_eq!(next.page_token.as_deref(), Some("cursor-2"));
    }
}
