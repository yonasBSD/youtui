use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{ContinuationParams, YoutubeID};
use crate::continuations::Continuable;
use crate::parse::ParseFrom;
use std::borrow::Cow;
use std::vec::Vec;

/// Query that will get continuations for a query that returned paginated
/// results.
pub struct GetContinuationsQuery<'a, Q> {
    query: &'a Q,
    continuation_params: ContinuationParams<'static>,
}

impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new<T: Continuable<Q>>(
        res: &'_ mut T,
        query: &'a Q,
    ) -> Option<GetContinuationsQuery<'a, Q>> {
        let continuation_params = res.take_continuation_params()?;
        Some(GetContinuationsQuery {
            continuation_params,
            query,
        })
    }
    /// Create a GetContinuationsQuery with dummy continuation params - for
    /// testing purposes.
    pub fn new_mock_unchecked(query: &'a Q) -> GetContinuationsQuery<'a, Q> {
        GetContinuationsQuery {
            query,
            continuation_params: ContinuationParams::from_raw(""),
        }
    }
}

impl<Q: Query<A>, A: AuthToken> Query<A> for GetContinuationsQuery<'_, Q>
where
    Q: PostQuery,
    Q::Output: ParseFrom<Self>,
{
    type Output = Q::Output;
    type Method = PostMethod;
}

impl<Q> PostQuery for GetContinuationsQuery<'_, Q>
where
    Q: PostQuery,
{
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        self.query.header()
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        let params = self.continuation_params.get_raw();
        vec![("ctoken", params.into()), ("continuation", params.into())]
    }
    fn path(&self) -> &str {
        self.query.path()
    }
}
