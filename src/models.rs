use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateBoardRequest {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Post {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePostRequest {
    pub board_id: Uuid,
    pub title: String,
    pub content: String,
    pub author: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub author: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub post_id: Uuid,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    /// Page number (starting from 1)
    #[serde(default = "default_page")]
    #[schema(default = 1, minimum = 1)]
    pub page: u32,
    /// Number of items per page
    #[serde(default = "default_limit")]
    #[schema(default = 10, minimum = 1, maximum = 100)]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    10
}

/// Metadata about pagination
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMeta {
    /// Current page number
    pub page: u32,
    /// Number of items per page
    pub limit: u32,
    /// Total number of items (if available)
    pub total: Option<u32>, // Optional as count might be expensive
    /// Total number of pages (if total is available)
    pub total_pages: Option<u32>,
}

/// Wrapper for paginated responses
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Pagination metadata
    pub meta: PaginationMeta,
    /// The data for the current page
    pub data: Vec<T>,
}

/// For metrics and health checks
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}


