use actix_web::{get, post, web, HttpResponse, Responder};
use scylla::Session;
use chrono::{TimeZone, Utc};
use uuid::Uuid;
use std::time::Instant;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::models::{
    Board, CreateBoardRequest, 
    Post, CreatePostRequest, 
    Comment, CreateCommentRequest,
    HealthResponse
};

// For metrics
static REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);
static DB_REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);

// Health check endpoint
/// Check API health
///
/// Returns health status, version, and timestamp
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "API health status", body = HealthResponse)
    )
)]
#[get("/health")]
pub async fn health_check() -> impl Responder {
    let response = HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    };

    HttpResponse::Ok().json(response)
}

// Metrics endpoint for Prometheus
/// Get API metrics for Prometheus
///
/// Returns plain text metrics in Prometheus format
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain")
    )
)]
#[get("/metrics")]
pub async fn metrics() -> impl Responder {
    let metrics_text = format!(
        "# HELP api_requests_total Total number of API requests\n\
         # TYPE api_requests_total counter\n\
         api_requests_total {}\n\
         # HELP db_requests_total Total number of database requests\n\
         # TYPE db_requests_total counter\n\
         db_requests_total {}\n",
        REQUEST_COUNT.load(Ordering::Relaxed),
        DB_REQUEST_COUNT.load(Ordering::Relaxed)
    );

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(metrics_text)
}

// Board related endpoints
/// Create a new board
///
/// Creates a new discussion board with the provided data
#[utoipa::path(
    post,
    path = "/boards",
    request_body = CreateBoardRequest,
    responses(
        (status = 201, description = "Board created successfully", body = Board),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/boards")]
pub async fn create_board(
    session: web::Data<Arc<Session>>,
    board_data: web::Json<CreateBoardRequest>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    
    let start = Instant::now();
    let board = Board {
        id: Uuid::new_v4(),
        name: board_data.name.clone(),
        description: board_data.description.clone(),
        created_at: Utc::now(),
    };
    
    let prepared = session.prepare("INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)").await;
    
    if let Err(e) = prepared {
        return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
    }
    
    // Convert DateTime to the format supported by Scylla (unix timestamp in milliseconds)
    let created_at_timestamp = board.created_at.timestamp_millis();
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session
        .execute(
            &prepared.unwrap(),
            (board.id, &board.name, &board.description, created_at_timestamp),
        )
        .await;

    let duration = start.elapsed().as_millis();

    match result {
        Ok(_) => {
            // Add response header with processing time for monitoring
            HttpResponse::Created()
                .append_header(("X-Processing-Time-Ms", duration.to_string()))
                .json(board)
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating board: {}", e)),
    }
}

/// Get all boards
///
/// Returns a list of all discussion boards
#[utoipa::path(
    get,
    path = "/boards",
    responses(
        (status = 200, description = "List of boards retrieved successfully", body = Vec<Board>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/boards")]
pub async fn get_boards(session: web::Data<Arc<Session>>) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    let prepared = match session.prepare("SELECT id, name, description, created_at FROM boards").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session.execute(&prepared, &[]).await;
    
    let duration = start.elapsed().as_millis();
    
    match result {
        Ok(rows) => {
            let boards: Vec<Board> = rows
                .rows
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| {
                    let id = row.columns[0].as_ref()?.as_uuid()?;
                    let name = row.columns[1].as_ref()?.as_text()?.to_string();
                    let description = row.columns[2].as_ref()?.as_text()?.to_string();
                    
                    let timestamp_millis = row.columns[3].as_ref()?.as_bigint()?;
                    let created_at = chrono::Utc.timestamp_millis_opt(timestamp_millis).single()?;

                    Some(Board {
                        id,
                        name,
                        description,
                        created_at,
                    })
                })
                .collect();

            HttpResponse::Ok()
                .append_header(("X-Processing-Time-Ms", duration.to_string()))
                .json(boards)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching boards: {}", e)),
    }
}

/// Get board by ID
///
/// Returns a single board with the specified ID
#[utoipa::path(
    get,
    path = "/boards/{board_id}",
    params(
        ("board_id" = uuid::Uuid, Path, description = "Board ID")
    ),
    responses(
        (status = 200, description = "Board retrieved successfully", body = Board),
        (status = 404, description = "Board not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/boards/{board_id}")]
pub async fn get_board(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    let board_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, name, description, created_at FROM boards WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session.execute(&prepared, (board_id,)).await;
    
    let duration = start.elapsed().as_millis();
    
    match result {
        Ok(rows) => {
            if let Some(row) = rows.first_row_typed::<(Uuid, String, String, i64)>().ok() {
                let (id, name, description, created_at_ts) = row;
                let created_at = Utc.timestamp_millis_opt(created_at_ts).single()
                    .unwrap_or_else(|| Utc::now());
                
                let board = Board {
                    id,
                    name,
                    description,
                    created_at,
                };
                
                HttpResponse::Ok()
                    .append_header(("X-Processing-Time-Ms", duration.to_string()))
                    .json(board)
            } else {
                HttpResponse::NotFound().body(format!("Board with id {} not found", board_id))
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching board: {}", e)),
    }
}

// Post related endpoints
/// Create a new post
///
/// Creates a new post on a specific board
#[utoipa::path(
    post,
    path = "/posts",
    request_body = CreatePostRequest,
    responses(
        (status = 201, description = "Post created successfully", body = Post),
        (status = 400, description = "Board not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/posts")]
pub async fn create_post(
    session: web::Data<Arc<Session>>,
    post_data: web::Json<CreatePostRequest>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    // First check if the board exists
    let board_check = match session.prepare("SELECT id FROM boards WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let board_result = session.execute(&board_check, (post_data.board_id,)).await;
    
    match board_result {
        Ok(rows) => {
            if rows.rows.unwrap_or_default().is_empty() {
                return HttpResponse::BadRequest().body(format!("Board with id {} not found", post_data.board_id));
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error checking board: {}", e)),
    }
    
    let post = Post {
        id: Uuid::new_v4(),
        board_id: post_data.board_id,
        title: post_data.title.clone(),
        content: post_data.content.clone(),
        created_at: Utc::now(),
        author: post_data.author.clone(),
    };
    
    let prepared = match session.prepare("INSERT INTO posts (id, board_id, title, content, created_at, author) VALUES (?, ?, ?, ?, ?, ?)").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    // Convert DateTime to the format supported by Scylla (unix timestamp in milliseconds)
    let created_at_timestamp = post.created_at.timestamp_millis();
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session
        .execute(
            &prepared,
            (post.id, post.board_id, &post.title, &post.content, created_at_timestamp, &post.author),
        )
        .await;

    let duration = start.elapsed().as_millis();

    match result {
        Ok(_) => HttpResponse::Created()
            .append_header(("X-Processing-Time-Ms", duration.to_string()))
            .json(post),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating post: {}", e)),
    }
}

/// Get posts by board
///
/// Returns all posts for a specific board
#[utoipa::path(
    get,
    path = "/boards/{board_id}/posts",
    params(
        ("board_id" = uuid::Uuid, Path, description = "Board ID")
    ),
    responses(
        (status = 200, description = "Posts retrieved successfully", body = Vec<Post>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/boards/{board_id}/posts")]
pub async fn get_posts_by_board(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    let board_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, board_id, title, content, created_at, author FROM posts WHERE board_id = ? ALLOW FILTERING").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session.execute(&prepared, (board_id,)).await;
    
    let duration = start.elapsed().as_millis();
    
    match result {
        Ok(rows) => {
            let posts: Vec<Post> = rows
                .rows
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| {
                    let id = row.columns[0].as_ref()?.as_uuid()?;
                    let board_id = row.columns[1].as_ref()?.as_uuid()?;
                    let title = row.columns[2].as_ref()?.as_text()?.to_string();
                    let content = row.columns[3].as_ref()?.as_text()?.to_string();
                    let timestamp_millis = row.columns[4].as_ref()?.as_bigint()?;
                    let author = row.columns[5].as_ref()?.as_text()?.to_string();
                    
                    let created_at = chrono::Utc.timestamp_millis_opt(timestamp_millis).single()?;

                    Some(Post {
                        id,
                        board_id,
                        title,
                        content,
                        created_at,
                        author,
                    })
                })
                .collect();

            HttpResponse::Ok()
                .append_header(("X-Processing-Time-Ms", duration.to_string()))
                .json(posts)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching posts: {}", e)),
    }
}

/// Get post by ID
///
/// Returns a single post with the specified ID
#[utoipa::path(
    get,
    path = "/posts/{post_id}",
    params(
        ("post_id" = uuid::Uuid, Path, description = "Post ID")
    ),
    responses(
        (status = 200, description = "Post retrieved successfully", body = Post),
        (status = 404, description = "Post not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/posts/{post_id}")]
pub async fn get_post(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    let post_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, board_id, title, content, created_at, author FROM posts WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session.execute(&prepared, (post_id,)).await;
    
    let duration = start.elapsed().as_millis();
    
    match result {
        Ok(rows) => {
            if let Some(row) = rows.first_row() {
                let id = row.columns[0].as_ref().and_then(|c| c.as_uuid()).ok_or("Invalid UUID");
                let board_id = row.columns[1].as_ref().and_then(|c| c.as_uuid()).ok_or("Invalid board_id");
                let title = row.columns[2].as_ref().and_then(|c| c.as_text()).ok_or("Invalid title");
                let content = row.columns[3].as_ref().and_then(|c| c.as_text()).ok_or("Invalid content");
                let timestamp = row.columns[4].as_ref().and_then(|c| c.as_bigint()).ok_or("Invalid timestamp");
                let author = row.columns[5].as_ref().and_then(|c| c.as_text()).ok_or("Invalid author");
                
                if let (Ok(id), Ok(board_id), Ok(title), Ok(content), Ok(timestamp), Ok(author)) = 
                    (id, board_id, title, content, timestamp, author) {
                    let created_at = Utc.timestamp_millis_opt(*timestamp).single()
                        .unwrap_or_else(|| Utc::now());
                    
                    let post = Post {
                        id: *id,
                        board_id: *board_id,
                        title: title.to_string(),
                        content: content.to_string(),
                        created_at,
                        author: author.to_string(),
                    };
                    
                    return HttpResponse::Ok()
                        .append_header(("X-Processing-Time-Ms", duration.to_string()))
                        .json(post);
                }
            }
            
            HttpResponse::NotFound().body(format!("Post with id {} not found", post_id))
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching post: {}", e)),
    }
}

// Comment related endpoints
/// Create a new comment
///
/// Creates a new comment on a specific post
#[utoipa::path(
    post,
    path = "/comments",
    request_body = CreateCommentRequest,
    responses(
        (status = 201, description = "Comment created successfully", body = Comment),
        (status = 400, description = "Post not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/comments")]
pub async fn create_comment(
    session: web::Data<Arc<Session>>,
    comment_data: web::Json<CreateCommentRequest>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    // First check if the post exists
    let post_check = match session.prepare("SELECT id FROM posts WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let post_result = session.execute(&post_check, (comment_data.post_id,)).await;
    
    match post_result {
        Ok(rows) => {
            if rows.rows.unwrap_or_default().is_empty() {
                return HttpResponse::BadRequest().body(format!("Post with id {} not found", comment_data.post_id));
            }
        },
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error checking post: {}", e)),
    }
    
    let comment = Comment {
        id: Uuid::new_v4(),
        post_id: comment_data.post_id,
        content: comment_data.content.clone(),
        created_at: Utc::now(),
        author: comment_data.author.clone(),
    };
    
    let prepared = match session.prepare("INSERT INTO comments (id, post_id, content, created_at, author) VALUES (?, ?, ?, ?, ?)").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    // Convert DateTime to the format supported by Scylla (unix timestamp in milliseconds)
    let created_at_timestamp = comment.created_at.timestamp_millis();
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session
        .execute(
            &prepared,
            (comment.id, comment.post_id, &comment.content, created_at_timestamp, &comment.author),
        )
        .await;

    let duration = start.elapsed().as_millis();

    match result {
        Ok(_) => HttpResponse::Created()
            .append_header(("X-Processing-Time-Ms", duration.to_string()))
            .json(comment),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating comment: {}", e)),
    }
}

/// Get comments by post
///
/// Returns all comments for a specific post
#[utoipa::path(
    get,
    path = "/posts/{post_id}/comments",
    params(
        ("post_id" = uuid::Uuid, Path, description = "Post ID")
    ),
    responses(
        (status = 200, description = "Comments retrieved successfully", body = Vec<Comment>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/posts/{post_id}/comments")]
pub async fn get_comments_by_post(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    
    let post_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, post_id, content, created_at, author FROM comments WHERE post_id = ? ALLOW FILTERING").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    DB_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    let result = session.execute(&prepared, (post_id,)).await;
    
    let duration = start.elapsed().as_millis();
    
    match result {
        Ok(rows) => {
            let comments: Vec<Comment> = rows
                .rows
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| {
                    let id = row.columns[0].as_ref()?.as_uuid()?;
                    let post_id = row.columns[1].as_ref()?.as_uuid()?;
                    let content = row.columns[2].as_ref()?.as_text()?.to_string();
                    let timestamp_millis = row.columns[3].as_ref()?.as_bigint()?;
                    let author = row.columns[4].as_ref()?.as_text()?.to_string();
                    
                    let created_at = chrono::Utc.timestamp_millis_opt(timestamp_millis).single()?;

                    Some(Comment {
                        id,
                        post_id,
                        content,
                        created_at,
                        author,
                    })
                })
                .collect();

            HttpResponse::Ok()
                .append_header(("X-Processing-Time-Ms", duration.to_string()))
                .json(comments)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching comments: {}", e)),
    }
}

/// Intentionally slow endpoint
///
/// This endpoint is intentionally slow to demonstrate alerts
#[utoipa::path(
    get,
    path = "/slow",
    responses(
        (status = 200, description = "Slow endpoint response")
    )
)]
#[get("/slow")]
pub async fn slow_endpoint() -> impl Responder {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    
    // Simulate slow processing
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
    
    HttpResponse::Ok().body("This endpoint is intentionally slow")
}
