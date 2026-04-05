#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct QueryWriter {
    pairs: Vec<(String, String)>,
}

#[allow(dead_code)]
impl QueryWriter {
    pub(crate) fn push<T>(&mut self, key: &'static str, value: T)
    where
        T: ToString,
    {
        self.pairs.push((key.to_owned(), value.to_string()));
    }

    pub(crate) fn push_opt<T>(&mut self, key: &'static str, value: Option<T>)
    where
        T: ToString,
    {
        if let Some(value) = value {
            self.push(key, value);
        }
    }

    pub(crate) fn push_csv<I, T>(&mut self, key: &'static str, values: I)
    where
        I: IntoIterator<Item = T>,
        T: ToString,
    {
        let value = values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");

        if !value.is_empty() {
            self.push(key, value);
        }
    }

    pub(crate) fn finish(self) -> Vec<(String, String)> {
        self.pairs
    }
}

#[cfg(test)]
mod tests {
    use super::QueryWriter;

    #[test]
    fn push_adds_query_pair() {
        let mut query = QueryWriter::default();

        query.push("status", "active");

        assert_eq!(
            query.finish(),
            vec![("status".to_owned(), "active".to_owned())]
        );
    }

    #[test]
    fn push_csv_preserves_input_order_in_single_query_pair() {
        let mut query = QueryWriter::default();

        query.push_csv("symbols", ["AAPL", "MSFT", "TSLA"]);

        assert_eq!(
            query.finish(),
            vec![("symbols".to_owned(), "AAPL,MSFT,TSLA".to_owned())]
        );
    }

    #[test]
    fn push_csv_omits_empty_iterables() {
        let mut query = QueryWriter::default();

        query.push_csv("symbols", std::iter::empty::<&str>());

        assert!(query.finish().is_empty());
    }
}
