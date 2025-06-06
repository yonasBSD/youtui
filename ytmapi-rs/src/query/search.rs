use super::*;
use crate::common::SearchSuggestion;
use crate::parse::SearchResults;
pub use filteredsearch::*;
use std::borrow::Cow;

pub mod filteredsearch;

const SPECIALIZED_PLAYLIST_EXACT_MATCH_PARAMS: &str = "BagwQDhAKEAMQBBAJEAU%3D";
const SPECIALIZED_PLAYLIST_WITH_SUGGESTIONS_PARAMS: &str = "BQgIIAWoMEA4QChADEAQQCRAF";
const SPECIALIZED_PLAYLIST_PREFIX_PARAMS: &str = "EgeKAQQoA";
const SEARCH_QUERY_PATH: &str = "search";

// TODO Seal
// TODO: Add relevant parameters.
// Implements Default to allow simple implementation of Into<SearchQuery<S>>
pub trait SearchType: Default {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<str>>;
}

// Trait constraint - to simplify implementation of Query for BasicSearch,
// LibrarySearch and UploadSearch.
pub trait UnfilteredSearchType: SearchType {}

/// An API search query.
#[derive(PartialEq, Debug, Clone)]
pub struct SearchQuery<'a, S: SearchType> {
    query: Cow<'a, str>,
    spelling_mode: SpellingMode,
    search_type: S,
}

/// Whether or not to allow Google to attempt to auto correct spelling as part
/// of the results. Has no affect on Uploads or Library.
// XXX: May actually affect Library. To confirm.
#[derive(PartialEq, Debug, Clone, Default)]
pub enum SpellingMode {
    // My personal preference is to use ExactMatch by default, so that's what I've set.
    // Google's is WithSuggestions.
    #[default]
    ExactMatch,
    WithSuggestions,
}

/// Helper struct for SearchQuery type state pattern.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct BasicSearch;
/// Helper struct for SearchQuery type state pattern.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct LibrarySearch;
/// Helper struct for SearchQuery type state pattern.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct UploadSearch;

impl SearchType for BasicSearch {
    fn specialised_params(&self, spelling_mode: &SpellingMode) -> Option<Cow<str>> {
        match spelling_mode {
            SpellingMode::ExactMatch => Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D".into()),
            SpellingMode::WithSuggestions => None,
        }
    }
}
impl SearchType for UploadSearch {
    fn specialised_params(&self, _: &SpellingMode) -> Option<Cow<str>> {
        // TODO: Investigate if spelling suggestions take affect here.
        Some("agIYAw%3D%3D".into())
    }
}
impl SearchType for LibrarySearch {
    fn specialised_params(&self, _: &SpellingMode) -> Option<Cow<str>> {
        // XXX: It may be possible to actually filter these, see sigma67/ytmusicapi for
        // details. TODO: Investigate if spelling suggestions take affect here.
        Some("agIYBA%3D%3D".into())
    }
}

impl UnfilteredSearchType for BasicSearch {}
impl UnfilteredSearchType for UploadSearch {}
impl UnfilteredSearchType for LibrarySearch {}

impl<S: UnfilteredSearchType, A: AuthToken> Query<A> for SearchQuery<'_, S> {
    type Output = SearchResults;
    type Method = PostMethod;
}
impl<S: UnfilteredSearchType> PostQuery for SearchQuery<'_, S> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        search_query_header(self)
    }
    fn path(&self) -> &str {
        SEARCH_QUERY_PATH
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

// This currently requires type annotations.
// By default, uses SpellingMode exactmatch.
impl<'a, Q: Into<Cow<'a, str>>, S: SearchType> From<Q> for SearchQuery<'a, S> {
    fn from(value: Q) -> SearchQuery<'a, S> {
        SearchQuery {
            query: value.into(),
            spelling_mode: SpellingMode::default(),
            search_type: S::default(),
        }
    }
}

// By default, uses SpellingMode exactmatch.
impl<'a> SearchQuery<'a, BasicSearch> {
    pub fn new<Q: Into<Cow<'a, str>>>(q: Q) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: q.into(),
            spelling_mode: SpellingMode::default(),
            search_type: BasicSearch {},
        }
    }
}

impl<'a, S: SearchType> SearchQuery<'a, S> {
    /// Set spelling mode.
    pub fn with_spelling_mode(mut self, spelling_mode: SpellingMode) -> Self {
        self.spelling_mode = spelling_mode;
        self
    }
    /// Chnage the set query.
    pub fn with_query<Q: Into<Cow<'a, str>>>(mut self, query: Q) -> Self {
        self.query = query.into();
        self
    }
}

impl<'a> SearchQuery<'a, BasicSearch> {
    /// Apply a filter to the search. May change type of results returned.
    pub fn with_filter<F: FilteredSearchType>(
        self,
        filter: F,
    ) -> SearchQuery<'a, FilteredSearch<F>> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: FilteredSearch { filter },
        }
    }
    /// Search only uploads.
    pub fn uploads(self) -> SearchQuery<'a, UploadSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: UploadSearch,
        }
    }
    /// Search only library.
    pub fn library(self) -> SearchQuery<'a, LibrarySearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: LibrarySearch,
        }
    }
}

impl<'a, F: FilteredSearchType> SearchQuery<'a, FilteredSearch<F>> {
    /// Apply a filter to the search. May change type of results returned.
    pub fn with_filter<F2: FilteredSearchType>(
        self,
        filter: F2,
    ) -> SearchQuery<'a, FilteredSearch<F2>> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: FilteredSearch { filter },
        }
    }
    /// Remove filter from the query.
    pub fn unfiltered(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: BasicSearch,
        }
    }
}

impl<'a> SearchQuery<'a, UploadSearch> {
    /// Change scope to search generally instead of Uploads.
    pub fn with_scope_public(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: BasicSearch,
        }
    }
}
impl<'a> SearchQuery<'a, LibrarySearch> {
    /// Change scope to search generally instead of Library.
    pub fn with_scope_public(self) -> SearchQuery<'a, BasicSearch> {
        SearchQuery {
            query: self.query,
            spelling_mode: self.spelling_mode,
            search_type: BasicSearch,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct GetSearchSuggestionsQuery<'a> {
    query: Cow<'a, str>,
}

impl<'a> GetSearchSuggestionsQuery<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(value: S) -> GetSearchSuggestionsQuery<'a> {
        GetSearchSuggestionsQuery {
            query: value.into(),
        }
    }
}

impl<'a, S: Into<Cow<'a, str>>> From<S> for GetSearchSuggestionsQuery<'a> {
    fn from(value: S) -> GetSearchSuggestionsQuery<'a> {
        GetSearchSuggestionsQuery::new(value)
    }
}

impl<A: AuthToken> Query<A> for GetSearchSuggestionsQuery<'_> {
    type Output = Vec<SearchSuggestion>;
    type Method = PostMethod;
}
impl PostQuery for GetSearchSuggestionsQuery<'_> {
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        let value = self.query.as_ref().into();
        serde_json::Map::from_iter([("input".into(), value)])
    }
    fn path(&self) -> &str {
        "music/get_search_suggestions"
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        vec![]
    }
}

fn search_query_header<S: SearchType>(
    query: &SearchQuery<S>,
) -> serde_json::Map<String, serde_json::Value> {
    let value = query.query.as_ref().into();
    let params = search_query_params(query);
    if let Some(params) = params {
        serde_json::Map::from_iter([
            ("query".to_string(), value),
            ("params".to_string(), params.into()),
        ])
    } else {
        serde_json::Map::from_iter([("query".to_string(), value)])
    }
}
fn search_query_params<'a, S: SearchType>(query: &'a SearchQuery<'a, S>) -> Option<Cow<'a, str>> {
    query.search_type.specialised_params(&query.spelling_mode)
}
