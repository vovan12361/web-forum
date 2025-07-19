use utoipa::{OpenApi, openapi};
use crate::models::{
    Board, CreateBoardRequest,
    Post, CreatePostRequest,
    Comment, CreateCommentRequest,
    HealthResponse,
};

/// Generate OpenAPI documentation for our REST API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health_check,
        crate::routes::create_board,
        crate::routes::get_boards,
        crate::routes::get_board,
        crate::routes::create_post,
        crate::routes::get_posts_by_board,
        crate::routes::get_post,
        crate::routes::create_comment,
        crate::routes::get_comments_by_post,
        crate::routes::slow_endpoint,
    ),
    components(
        schemas(
            Board, 
            CreateBoardRequest, 
            Post, 
            CreatePostRequest, 
            Comment, 
            CreateCommentRequest, 
            HealthResponse
        )
    ),
    info(
        title = "Forum API",
        version = "1.0.0",
        description = "REST API for a forum service with boards, posts, and comments",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    )
)]
pub struct ApiDoc; 