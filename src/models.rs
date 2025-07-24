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

// For metrics and health checks
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

