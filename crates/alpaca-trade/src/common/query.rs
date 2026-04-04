#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct QueryWriter {
    pairs: Vec<(String, String)>,
}

#[allow(dead_code)]
impl QueryWriter {
    pub(crate) fn push_opt<T>(&mut self, key: &'static str, value: Option<T>)
    where
        T: ToString,
    {
        if let Some(value) = value {
            self.pairs.push((key.to_owned(), value.to_string()));
        }
    }

    pub(crate) fn finish(self) -> Vec<(String, String)> {
        self.pairs
    }
}
