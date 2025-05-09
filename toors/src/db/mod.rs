//! Database access layer for Toors

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    /// Error caused by a failed HTTP request.
    #[error("Request error caused by: {0}")]
    Request(String),
}

/// A type alias for a `Result` with the associated `DbError` enum.
pub type Result<T> = std::result::Result<T, DbError>;

/// Struct representing a GraphQL response.
#[derive(Deserialize, Serialize)]
struct DbResponse<T> {
    data: T,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Query<'a, T>
where
    T: Serialize + 'a,
{
    pub query: &'a str,
    pub variables: T,
}

// async fn query<'a, T, Q>(query: Query<'a, Q>) -> Result<T>
// where
//     T: DeserializeOwned,
//     Q: Serialize + 'a,
// {
//     &query
// }
