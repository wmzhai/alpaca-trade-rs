use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Endpoint {
    Account,
    Calendar,
    Clock,
}

impl Endpoint {
    pub(crate) fn path(&self) -> Cow<'_, str> {
        match self {
            Self::Account => Cow::Borrowed("/v2/account"),
            Self::Calendar => Cow::Borrowed("/v2/calendar"),
            Self::Clock => Cow::Borrowed("/v2/clock"),
        }
    }

    pub(crate) fn requires_auth(&self) -> bool {
        match self {
            Self::Account | Self::Calendar | Self::Clock => true,
        }
    }
}
