use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Endpoint {
    Account,
}

impl Endpoint {
    pub(crate) fn path(&self) -> Cow<'_, str> {
        match self {
            Self::Account => Cow::Borrowed("/v2/account"),
        }
    }

    pub(crate) fn requires_auth(&self) -> bool {
        match self {
            Self::Account => true,
        }
    }
}
