use std::future::Future;

use crate::error::Error;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Page<T, Request> {
    pub(crate) items: Vec<T>,
    pub(crate) next_request: Option<Request>,
}

#[allow(dead_code)]
pub(crate) trait PaginatedRequest: Clone {
    type Item;
}

#[allow(dead_code)]
pub(crate) async fn collect_all<Request, Fetch, FutureOutput>(
    initial_request: Request,
    mut fetch_page: Fetch,
) -> Result<Vec<Request::Item>, Error>
where
    Request: PaginatedRequest,
    Fetch: FnMut(Request) -> FutureOutput,
    FutureOutput: Future<Output = Result<Page<Request::Item, Request>, Error>>,
{
    let mut items = Vec::new();
    let mut next_request = Some(initial_request);

    while let Some(request) = next_request {
        let page = fetch_page(request).await?;
        items.extend(page.items);
        next_request = page.next_request;
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{Page, PaginatedRequest, collect_all};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DummyRequest {
        page: u8,
    }

    impl PaginatedRequest for DummyRequest {
        type Item = &'static str;
    }

    #[tokio::test]
    async fn collect_all_accumulates_items_until_no_next_page() {
        let items = collect_all(DummyRequest { page: 1 }, |request| async move {
            match request.page {
                1 => Ok(Page {
                    items: vec!["AAPL", "MSFT"],
                    next_request: Some(DummyRequest { page: 2 }),
                }),
                2 => Ok(Page {
                    items: vec!["TSLA"],
                    next_request: None,
                }),
                _ => unreachable!("test only covers two pages"),
            }
        })
        .await
        .expect("pagination should succeed");

        assert_eq!(items, vec!["AAPL", "MSFT", "TSLA"]);
    }
}
