use actix_web::{get, post, web, HttpResponse, Responder};
use scylla::Session;
use chrono::{TimeZone, Utc};
use uuid::Uuid;
use std::time::Instant;
use std::sync::Arc;
use prometheus::{Counter, Histogram, HistogramOpts, Registry, TextEncoder, opts};
use std::sync::OnceLock;
use scylla::frame::value::CqlTimestamp;

use crate::models::{
    Board, CreateBoardRequest, 
    Post, CreatePostRequest, 
    Comment, CreateCommentRequest,
    HealthResponse
};

// Prometheus metrics
static METRICS_REGISTRY: OnceLock<Registry> = OnceLock::new();
static REQUEST_COUNTER: OnceLock<Counter> = OnceLock::new();
static DB_REQUEST_COUNTER: OnceLock<Counter> = OnceLock::new();
static HTTP_REQUEST_DURATION: OnceLock<Histogram> = OnceLock::new();

fn init_metrics() -> &'static Registry {
    METRICS_REGISTRY.get_or_init(|| {
        let registry = Registry::new();
        
        let request_counter = Counter::with_opts(opts!(
            "api_requests_total",
            "Total number of API requests"
        )).unwrap();
        
        let db_request_counter = Counter::with_opts(opts!(
            "db_requests_total", 
            "Total number of database requests"
        )).unwrap();
        
        let http_duration_histogram = Histogram::with_opts(HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])).unwrap();
        
        registry.register(Box::new(request_counter.clone())).unwrap();
        registry.register(Box::new(db_request_counter.clone())).unwrap();
        registry.register(Box::new(http_duration_histogram.clone())).unwrap();
        
        REQUEST_COUNTER.set(request_counter).unwrap();
        DB_REQUEST_COUNTER.set(db_request_counter).unwrap();
        HTTP_REQUEST_DURATION.set(http_duration_histogram).unwrap();
        
        registry
    })
}

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
    let registry = init_metrics();
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(metrics) => HttpResponse::Ok()
            .content_type("text/plain")
            .body(metrics),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Error encoding metrics: {}", e))
    }
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let board = Board {
        id: Uuid::new_v4(),
        name: board_data.name.clone(),
        description: board_data.description.clone(),
        created_at: Utc::now(),
    };
    
    let prepared = session.prepare("INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)").await;
    
    if let Err(e) = prepared {
        let duration = start.elapsed().as_secs_f64();
        HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
        return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
    }
    
    // Use CqlTimestamp directly for ScyllaDB
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session
        .execute(
            &prepared.unwrap(),
            (board.id, &board.name, &board.description, CqlTimestamp(board.created_at.timestamp_millis())),
        )
        .await;

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    match result {
        Ok(_) => {
            HttpResponse::Created()
                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let prepared = match session.prepare("SELECT id, name, description, created_at FROM boards").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session.execute(&prepared, &[]).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
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
                    
                    // Try to get as CqlTimestamp first, fallback to bigint
                    let created_at = if let Some(cql_ts) = row.columns[3].as_ref().and_then(|c| c.as_cql_timestamp()) {
                        Utc.timestamp_millis_opt(cql_ts.0).single()?
                    } else if let Some(millis) = row.columns[3].as_ref().and_then(|c| c.as_bigint()) {
                        Utc.timestamp_millis_opt(millis).single()?
                    } else {
                        return None;
                    };

                    Some(Board {
                        id,
                        name,
                        description,
                        created_at,
                    })
                })
                .collect();

            HttpResponse::Ok()
                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let board_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, name, description, created_at FROM boards WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session.execute(&prepared, (board_id,)).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
    match result {
        Ok(rows) => {
            if let Some(row) = rows.first_row_typed::<(Uuid, String, String, CqlTimestamp)>().ok() {
                let (id, name, description, created_at_cql) = row;
                let created_at = Utc.timestamp_millis_opt(created_at_cql.0).single()
                    .unwrap_or_else(|| Utc::now());
                
                let board = Board {
                    id,
                    name,
                    description,
                    created_at,
                };
                
                HttpResponse::Ok()
                    .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    // First check if the board exists
    let board_check = match session.prepare("SELECT id FROM boards WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let board_result = session.execute(&board_check, (post_data.board_id,)).await;
    
    match board_result {
        Ok(rows) => {
            if rows.rows.unwrap_or_default().is_empty() {
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::BadRequest().body(format!("Board with id {} not found", post_data.board_id));
            }
        },
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error checking board: {}", e));
        }
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
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Use CqlTimestamp directly for ScyllaDB
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session
        .execute(
            &prepared,
            (post.id, post.board_id, &post.title, &post.content, CqlTimestamp(post.created_at.timestamp_millis()), &post.author),
        )
        .await;

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    match result {
        Ok(_) => HttpResponse::Created()
            .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let board_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, board_id, title, content, created_at, author FROM posts WHERE board_id = ? ALLOW FILTERING").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session.execute(&prepared, (board_id,)).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
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
                    let author = row.columns[5].as_ref()?.as_text()?.to_string();
                    
                    // Try to get as CqlTimestamp first, fallback to bigint
                    let created_at = if let Some(cql_ts) = row.columns[4].as_ref().and_then(|c| c.as_cql_timestamp()) {
                        Utc.timestamp_millis_opt(cql_ts.0).single()?
                    } else if let Some(millis) = row.columns[4].as_ref().and_then(|c| c.as_bigint()) {
                        Utc.timestamp_millis_opt(millis).single()?
                    } else {
                        return None;
                    };

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
                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let post_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, board_id, title, content, created_at, author FROM posts WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session.execute(&prepared, (post_id,)).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
    match result {
        Ok(rows) => {
            match rows.first_row() {
                Ok(row) => {
                    let id_res = row.columns[0].as_ref().and_then(|c| c.as_uuid());
                    let board_id_res = row.columns[1].as_ref().and_then(|c| c.as_uuid());
                    let title_res = row.columns[2].as_ref().and_then(|c| c.as_text());
                    let content_res = row.columns[3].as_ref().and_then(|c| c.as_text());
                    let author_res = row.columns[5].as_ref().and_then(|c| c.as_text());
                    
                    // Try to get timestamp as CqlTimestamp first, fallback to bigint
                    let created_at = if let Some(cql_ts) = row.columns[4].as_ref().and_then(|c| c.as_cql_timestamp()) {
                        Utc.timestamp_millis_opt(cql_ts.0).single().unwrap_or_else(|| Utc::now())
                    } else if let Some(millis) = row.columns[4].as_ref().and_then(|c| c.as_bigint()) {
                        Utc.timestamp_millis_opt(millis).single().unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now()
                    };
                    
                    if let (Some(id), Some(board_id), Some(title), Some(content), Some(author)) = 
                        (id_res, board_id_res, title_res, content_res, author_res) {
                        
                        let post = Post {
                            id,
                            board_id,
                            title: title.to_string(),
                            content: content.to_string(),
                            created_at,
                            author: author.to_string(),
                        };
                        
                        return HttpResponse::Ok()
                            .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
                            .json(post);
                    }
                },
                Err(_) => {}
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    // First check if the post exists
    let post_check = match session.prepare("SELECT id FROM posts WHERE id = ?").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let post_result = session.execute(&post_check, (comment_data.post_id,)).await;
    
    match post_result {
        Ok(rows) => {
            if rows.rows.unwrap_or_default().is_empty() {
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::BadRequest().body(format!("Post with id {} not found", comment_data.post_id));
            }
        },
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error checking post: {}", e));
        }
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
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Use CqlTimestamp directly for ScyllaDB
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session
        .execute(
            &prepared,
            (comment.id, comment.post_id, &comment.content, CqlTimestamp(comment.created_at.timestamp_millis()), &comment.author),
        )
        .await;

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    match result {
        Ok(_) => HttpResponse::Created()
            .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let post_id = path.into_inner();
    
    let prepared = match session.prepare("SELECT id, post_id, content, created_at, author FROM comments WHERE post_id = ? ALLOW FILTERING").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session.execute(&prepared, (post_id,)).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
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
                    let author = row.columns[4].as_ref()?.as_text()?.to_string();
                    
                    // Try to get as CqlTimestamp first, fallback to bigint
                    let created_at = if let Some(cql_ts) = row.columns[3].as_ref().and_then(|c| c.as_cql_timestamp()) {
                        Utc.timestamp_millis_opt(cql_ts.0).single()?
                    } else if let Some(millis) = row.columns[3].as_ref().and_then(|c| c.as_bigint()) {
                        Utc.timestamp_millis_opt(millis).single()?
                    } else {
                        return None;
                    };

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
                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
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
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    // Simulate slow processing
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
    
    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());
    
    HttpResponse::Ok().body("This endpoint is intentionally slow")
}
