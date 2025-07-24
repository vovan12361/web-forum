use actix_web::{get, post, web, HttpResponse, Responder, web::Query};
use scylla::{Session, prepared_statement::PreparedStatement};
use futures::stream::StreamExt;
use chrono::{TimeZone, Utc};
use uuid::Uuid;
use std::time::{Instant, Duration};
use std::sync::Arc;
use prometheus::{Counter, Histogram, HistogramOpts, Registry, TextEncoder, Gauge, opts};
use std::sync::OnceLock;
use tracing::{info, warn, error, debug, instrument};
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::models::{
    Board, CreateBoardRequest, 
    Post, CreatePostRequest, 
    Comment, CreateCommentRequest,
    HealthResponse, PaginationParams, PaginatedResponse, PaginationMeta
};

// Cache structure for performance optimization
#[derive(Clone)]
pub struct CacheEntry<T> {
    data: T,
    timestamp: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }

    pub fn get_data(&self) -> &T {
        &self.data
    }
}

// In-memory cache for frequently accessed data
pub type BoardsCache = Arc<RwLock<HashMap<String, CacheEntry<Vec<Board>>>>>;
pub type PostsCache = Arc<RwLock<HashMap<String, CacheEntry<Vec<Post>>>>>;

// Prepared statements for better performance
pub struct PreparedStatements {
    pub get_boards: PreparedStatement,
    pub get_board_by_id: PreparedStatement,
    pub create_board: PreparedStatement,
    pub get_posts_by_board: PreparedStatement,
    pub get_post_by_id: PreparedStatement,
    pub create_post: PreparedStatement,
    pub get_comments_by_post: PreparedStatement,
    pub create_comment: PreparedStatement,
}

static PREPARED_STATEMENTS: OnceLock<PreparedStatements> = OnceLock::new();
static BOARDS_CACHE: OnceLock<BoardsCache> = OnceLock::new();
static POSTS_CACHE: OnceLock<PostsCache> = OnceLock::new();

// Individual prepared statement references for easier access
static CREATE_BOARD_STMT: OnceLock<PreparedStatement> = OnceLock::new();
static GET_BOARDS_STMT: OnceLock<PreparedStatement> = OnceLock::new();
static GET_BOARD_STMT: OnceLock<PreparedStatement> = OnceLock::new();

// Prometheus metrics
static METRICS_REGISTRY: OnceLock<Registry> = OnceLock::new();
static REQUEST_COUNTER: OnceLock<Counter> = OnceLock::new();
static DB_REQUEST_COUNTER: OnceLock<Counter> = OnceLock::new();
static HTTP_REQUEST_DURATION: OnceLock<Histogram> = OnceLock::new();
static ACTIVE_REQUESTS: OnceLock<Gauge> = OnceLock::new();

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
        
        let active_requests_gauge = Gauge::with_opts(opts!(
            "http_requests_active",
            "Number of HTTP requests currently being processed"
        )).unwrap();
        
        registry.register(Box::new(request_counter.clone())).unwrap();
        registry.register(Box::new(db_request_counter.clone())).unwrap();
        registry.register(Box::new(http_duration_histogram.clone())).unwrap();
        registry.register(Box::new(active_requests_gauge.clone())).unwrap();
        
        REQUEST_COUNTER.set(request_counter).unwrap();
        DB_REQUEST_COUNTER.set(db_request_counter).unwrap();
        HTTP_REQUEST_DURATION.set(http_duration_histogram).unwrap();
        ACTIVE_REQUESTS.set(active_requests_gauge).unwrap();
        
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
#[instrument(name = "health_check", skip_all)]
pub async fn health_check() -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    debug!("Health check requested");
    let response = HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    };
    
    info!("Health check successful");
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
#[instrument(name = "metrics", skip_all)]
pub async fn metrics() -> impl Responder {
    debug!("Prometheus metrics requested");
    let registry = init_metrics();
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(metrics) => {
            debug!("Metrics encoded successfully, {} families", metric_families.len());
            HttpResponse::Ok()
                .content_type("text/plain")
                .body(metrics)
        },
        Err(e) => {
            error!("Error encoding metrics: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Error encoding metrics: {}", e))
        }
    }
}

// Function to initialize prepared statements
pub async fn init_prepared_statements(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let prepared = PreparedStatements {
        get_boards: session.prepare("SELECT id, name, description, created_at FROM boards").await?,
        get_board_by_id: session.prepare("SELECT id, name, description, created_at FROM boards WHERE id = ?").await?,
        create_board: session.prepare("INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)").await?,
        get_posts_by_board: session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE board_id = ? ALLOW FILTERING").await?,
        get_post_by_id: session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE id = ?").await?,
        create_post: session.prepare("INSERT INTO posts (id, board_id, title, content, author, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)").await?,
        get_comments_by_post: session.prepare("SELECT id, post_id, content, author, created_at FROM comments WHERE post_id = ? ALLOW FILTERING").await?,
        create_comment: session.prepare("INSERT INTO comments (id, post_id, content, author, created_at) VALUES (?, ?, ?, ?, ?)").await?,
    };
    
    // Set individual statements for easier access
    CREATE_BOARD_STMT.set(prepared.create_board.clone()).map_err(|_| "Failed to set create board statement")?;
    GET_BOARDS_STMT.set(prepared.get_boards.clone()).map_err(|_| "Failed to set get boards statement")?;
    GET_BOARD_STMT.set(prepared.get_board_by_id.clone()).map_err(|_| "Failed to set get board statement")?;
    
    PREPARED_STATEMENTS.set(prepared).map_err(|_| "Failed to set prepared statements")?;
    BOARDS_CACHE.set(Arc::new(RwLock::new(HashMap::new()))).map_err(|_| "Failed to set boards cache")?;
    POSTS_CACHE.set(Arc::new(RwLock::new(HashMap::new()))).map_err(|_| "Failed to set posts cache")?;
    
    info!("Prepared statements and caches initialized successfully");
    Ok(())
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
#[instrument(name = "create_board", skip(session), fields(board_name = %board_data.name))]
pub async fn create_board(
    session: web::Data<Arc<Session>>,
    board_data: web::Json<CreateBoardRequest>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    info!("Creating new board: {}", board_data.name);
        
        let board = Board {
            id: Uuid::new_v4(),
            name: board_data.name.clone(),
            description: board_data.description.clone(),
            created_at: Utc::now(),
        };
        
        debug!("Generated board ID: {}", board.id);
        
        // Use prepared statement for better performance
        let result = if let Some(stmt) = CREATE_BOARD_STMT.get() {
            session.execute(
                stmt,
                (board.id, &board.name, &board.description, board.created_at.timestamp_millis()),
            ).await
        } else {
            // Fallback to regular query if prepared statement not ready
            warn!("Prepared statement not available, using regular query");
            session.query(
                "INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)",
                (board.id, &board.name, &board.description, board.created_at.timestamp_millis()),
            ).await
        };
        
        DB_REQUEST_COUNTER.get().unwrap().inc();

        match result {
            Ok(_) => {
                info!("Board created successfully: {}", board.name);
                HttpResponse::Created().json(board)
            },
            Err(e) => {
                error!("Error creating board: {}", e);
                HttpResponse::InternalServerError().body(format!("Error creating board: {}", e))
            },
        }
}

/// Get all boards with pagination
///
/// Returns a paginated list of all discussion boards
#[utoipa::path(
    get,
    path = "/boards",
    params(
        ("page" = Option<u32>, Query, description = "Page number (starts at 1)", example = 1),
        ("limit" = Option<u32>, Query, description = "Number of items per page", example = 10)
    ),
    responses(
        (status = 200, description = "Paginated list of boards retrieved successfully", body = PaginatedResponse<Board>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/boards")]
#[instrument(name = "get_boards", skip(session))]
pub async fn get_boards(
    session: web::Data<Arc<Session>>,
    pagination: Query<PaginationParams>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    let page = pagination.page.max(1); // Ensure page >= 1
    let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

    info!("Fetching boards (page: {}, limit: {})", page, limit);
    let start = Instant::now();

    // Prepare statement with page size
    let mut prepared = match session.prepare("SELECT id, name, description, created_at FROM boards").await {
        Ok(stmt) => stmt,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Set page size for efficient pagination
    prepared.set_page_size(limit as i32);

    DB_REQUEST_COUNTER.get().unwrap().inc();
    
    // Use execute_iter for paginated results
    let row_iterator = match session.execute_iter(prepared, &[]).await {
        Ok(iterator) => iterator,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error executing query: {}", e));
        }
    };

    let mut boards = Vec::new();
    let mut total_fetched = 0u32;

    // Skip to the requested page
    let skip_count = (page - 1) * limit;
    let mut skipped = 0u32;

    // Convert iterator to stream and iterate through pages
    let mut rows_stream = row_iterator.into_typed::<(uuid::Uuid, String, String, i64)>();
    
    while let Some(next_row_res) = rows_stream.next().await {
        match next_row_res {
            Ok((id, name, description, created_at_millis)) => {
                // Skip rows until we reach the desired page
                if skipped < skip_count {
                    skipped += 1;
                    continue;
                }
                
                // Stop if we have enough items for this page
                if total_fetched >= limit {
                    break;
                }

                // Convert timestamp
                let created_at = match Utc.timestamp_millis_opt(created_at_millis).single() {
                    Some(dt) => dt,
                    None => {
                        warn!("Invalid timestamp for board {}: {}", id, created_at_millis);
                        continue;
                    }
                };

                boards.push(Board {
                    id,
                    name,
                    description,
                    created_at,
                });

                total_fetched += 1;
            },
            Err(e) => {
                error!("Error reading row: {}", e);
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
            }
        }
    }

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    // For pagination metadata, we'll estimate total pages
    // In a production system, you might want to maintain a separate count
    let has_more = total_fetched == limit; // If we got a full page, there might be more
    
    let meta = PaginationMeta {
        page,
        limit,
        total: None, // We don't have exact total count without additional query
        total_pages: if has_more { None } else { Some(page) }, // If no more data, current page is last
    };

    let response = PaginatedResponse {
        meta,
        data: boards,
    };

    info!("Successfully fetched {} boards (page: {}, limit: {}, duration: {}ms)", response.data.len(), page, limit, duration.as_millis());
    HttpResponse::Ok()
        .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
        .append_header(("X-Has-More", has_more.to_string()))
        .json(response)
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
#[instrument(name = "get_board", skip(session), fields(board_id = %path))]
pub async fn get_board(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    let board_id = path.into_inner();
    info!("Fetching board with ID: {}", board_id);
        
        // Check cache first
        let board_cache_key = board_id.to_string();
        if let Some(cached_board) = BOARDS_CACHE.get().unwrap().read().await.get(&board_cache_key) {
            if !cached_board.is_expired() {
                info!("Cache hit for board ID: {}", board_id);
                return HttpResponse::Ok().json(cached_board.get_data());
            } else {
                info!("Cache expired for board ID: {}, fetching fresh data", board_id);
            }
        } else {
            info!("No cache entry for board ID: {}, fetching data", board_id);
        }
        
        // Use prepared statement for better performance
        let result = if let Some(stmt) = GET_BOARD_STMT.get() {
            session.execute(stmt, (board_id,)).await
        } else {
            // Fallback to regular query if prepared statement not ready
            warn!("Prepared statement not available, using regular query");
            session.query("SELECT id, name, description, created_at FROM boards WHERE id = ?", (board_id,)).await
        };
        
        DB_REQUEST_COUNTER.get().unwrap().inc();
        
        match result {
            Ok(rows) => {
                if let Some(row) = rows.rows.as_ref().and_then(|r| r.first()) {
                    if let (Some(id), Some(name), Some(description)) = (
                        row.columns[0].as_ref().and_then(|c| c.as_uuid()),
                        row.columns[1].as_ref().and_then(|c| c.as_text()),
                        row.columns[2].as_ref().and_then(|c| c.as_text()),
                    ) {
                        // Handle bigint timestamps
                        let created_at = if let Some(millis) = row.columns[3].as_ref().and_then(|c| c.as_bigint()) {
                            Utc.timestamp_millis_opt(millis).single().unwrap_or_else(|| Utc::now())
                        } else {
                            Utc::now()
                        };
                        
                        let board = Board {
                            id,
                            name: name.to_string(),
                            description: description.to_string(),
                            created_at,
                        };
                        
                        // Update cache
                        let cache_entry = CacheEntry::new(vec![board.clone()], Duration::from_secs(300)); // 5 minutes TTL
                        BOARDS_CACHE.get().unwrap().write().await.insert(board_cache_key, cache_entry);

                        info!("Board found: {}", board.name);
                        return HttpResponse::Ok().json(board);
                    }
                }
                
                warn!("Board with id {} not found", board_id);
                HttpResponse::NotFound().body(format!("Board with id {} not found", board_id))
            }
            Err(e) => {
                error!("Error fetching board: {}", e);
                HttpResponse::InternalServerError().body(format!("Error fetching board: {}", e))
            },
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
#[instrument(name = "create_post", skip(session), fields(board_id = %post_data.board_id, title = %post_data.title, author = %post_data.author))]
pub async fn create_post(
    session: web::Data<Arc<Session>>,
    post_data: web::Json<CreatePostRequest>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    
    info!("Creating new post: '{}' by {} on board {}", post_data.title, post_data.author, post_data.board_id);
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    // First check if the board exists
    debug!("Checking if board exists: {}", post_data.board_id);
    let board_check = match session.prepare("SELECT id FROM boards WHERE id = ?").await {
        Ok(p) => {
            debug!("Board check query prepared successfully");
            p
        },
        Err(e) => {
            error!("Error preparing board check query: {}", e);
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
                warn!("Board with id {} not found", post_data.board_id);
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::BadRequest().body(format!("Board with id {} not found", post_data.board_id));
            } else {
                debug!("Board exists, proceeding with post creation");
            }
        },
        Err(e) => {
            error!("Error checking board existence: {}", e);
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error checking board: {}", e));
        }
    }
    
    let now = Utc::now();
    let post = Post {
        id: Uuid::new_v4(),
        board_id: post_data.board_id,
        title: post_data.title.clone(),
        content: post_data.content.clone(),
        created_at: now,
        updated_at: now,
        author: post_data.author.clone(),
    };
    
    debug!("Generated post ID: {}", post.id);
    
    let prepared = match session.prepare("INSERT INTO posts (id, board_id, title, content, author, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)").await {
        Ok(p) => {
            debug!("Post insert query prepared successfully");
            p
        },
        Err(e) => {
            error!("Error preparing post insert query: {}", e);
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Use timestamp_millis directly for ScyllaDB BIGINT
    DB_REQUEST_COUNTER.get().unwrap().inc();
    debug!("Executing post insert query");
    let result = session
        .execute(
            &prepared,
            (post.id, post.board_id, &post.title, &post.content, &post.author, post.created_at.timestamp_millis(), post.updated_at.timestamp_millis()),
        )
        .await;

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    match result {
        Ok(_) => {
            info!("Post created successfully: '{}' (duration: {}ms)", post.title, duration.as_millis());
            HttpResponse::Created()
                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
                .json(post)
        },
        Err(e) => {
            error!("Error creating post: {}", e);
            HttpResponse::InternalServerError().body(format!("Error creating post: {}", e))
        },
    }
}

/// Get posts by board with pagination
///
/// Returns paginated posts for a specific board using ScyllaDB native pagination
#[utoipa::path(
    get,
    path = "/boards/{board_id}/posts",
    params(
        ("board_id" = uuid::Uuid, Path, description = "Board ID"),
        ("page" = Option<u32>, Query, description = "Page number (starts at 1)", example = 1),
        ("limit" = Option<u32>, Query, description = "Number of items per page", example = 10)
    ),
    responses(
        (status = 200, description = "Paginated posts retrieved successfully", body = PaginatedResponse<Post>),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(name = "get_posts_by_board", skip(session), fields(board_id = %path))]
#[get("/boards/{board_id}/posts")]
pub async fn get_posts_by_board(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
    pagination: Query<PaginationParams>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    let board_id = path.into_inner();
    let page = pagination.page.max(1); // Ensure page >= 1
    let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

    info!("Fetching posts for board {} (page: {}, limit: {})", board_id, page, limit);
    let start = Instant::now();

    // Prepare statement with page size for efficient pagination
    let mut prepared = match session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE board_id = ? ALLOW FILTERING").await {
        Ok(stmt) => stmt,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Set page size for efficient pagination
    prepared.set_page_size(limit as i32);

    DB_REQUEST_COUNTER.get().unwrap().inc();
    
    // Use execute_iter for paginated results
    let row_iterator = match session.execute_iter(prepared, (board_id,)).await {
        Ok(iterator) => iterator,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error executing query: {}", e));
        }
    };

    let mut posts = Vec::new();
    let mut total_fetched = 0u32;

    // Skip to the requested page
    let skip_count = (page - 1) * limit;
    let mut skipped = 0u32;

    // Convert iterator to stream and iterate through pages
    let mut rows_stream = row_iterator.into_typed::<(uuid::Uuid, uuid::Uuid, String, String, String, i64, i64)>();
    
    while let Some(next_row_res) = rows_stream.next().await {
        match next_row_res {
            Ok((id, board_id, title, content, author, created_at_millis, updated_at_millis)) => {
                // Skip rows until we reach the desired page
                if skipped < skip_count {
                    skipped += 1;
                    continue;
                }
                
                // Stop if we have enough items for this page
                if total_fetched >= limit {
                    break;
                }

                // Convert timestamps
                let created_at = match Utc.timestamp_millis_opt(created_at_millis).single() {
                    Some(dt) => dt,
                    None => {
                        warn!("Invalid created_at timestamp for post {}: {}", id, created_at_millis);
                        continue;
                    }
                };
                
                let updated_at = match Utc.timestamp_millis_opt(updated_at_millis).single() {
                    Some(dt) => dt,
                    None => {
                        warn!("Invalid updated_at timestamp for post {}: {}", id, updated_at_millis);
                        continue;
                    }
                };

                posts.push(Post {
                    id,
                    board_id,
                    title,
                    content,
                    author,
                    created_at,
                    updated_at,
                });

                total_fetched += 1;
            },
            Err(e) => {
                error!("Error reading row: {}", e);
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
            }
        }
    }

    // Sort posts by created_at in descending order (newest first)
    posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    // For pagination metadata, we'll estimate total pages
    // In a production system, you might want to maintain a separate count
    let has_more = total_fetched == limit; // If we got a full page, there might be more
    
    let meta = PaginationMeta {
        page,
        limit,
        total: None, // We don't have exact total count without additional query
        total_pages: if has_more { None } else { Some(page) }, // If no more data, current page is last
    };

    let response = PaginatedResponse {
        meta,
        data: posts,
    };

    info!("Successfully fetched {} posts for board {} (page: {}, limit: {}, duration: {}ms)", response.data.len(), board_id, page, limit, duration.as_millis());
    HttpResponse::Ok()
        .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
        .append_header(("X-Has-More", has_more.to_string()))
        .json(response)
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
#[instrument(name = "get_post", skip(session), fields(post_id = %path))]
#[get("/posts/{post_id}")]
pub async fn get_post(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let post_id = path.into_inner();
    
    // Check cache first
    let post_cache_key = format!("post_{}", post_id);
    if let Some(cached_post) = POSTS_CACHE.get().unwrap().read().await.get(&post_cache_key) {
        if !cached_post.is_expired() {
            info!("Cache hit for post ID: {}", post_id);
            if let Some(post) = cached_post.get_data().first() {
                return HttpResponse::Ok().json(post);
            }
        } else {
            info!("Cache expired for post ID: {}, fetching fresh data", post_id);
        }
    } else {
        info!("No cache entry for post ID: {}, fetching data", post_id);
    }
    
    let prepared = match session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE id = ?").await {
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
                    let author_res = row.columns[4].as_ref().and_then(|c| c.as_text());
                    
                    // Handle bigint timestamps from database
                    let created_at = if let Some(millis) = row.columns[5].as_ref().and_then(|c| c.as_bigint()) {
                        Utc.timestamp_millis_opt(millis).single().unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now()
                    };

                    let updated_at = if let Some(millis) = row.columns[6].as_ref().and_then(|c| c.as_bigint()) {
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
                            updated_at,
                            author: author.to_string(),
                        };
                        
                        // Update cache
                        let cache_entry = CacheEntry::new(vec![post.clone()], Duration::from_secs(300)); // 5 minutes TTL
                        POSTS_CACHE.get().unwrap().write().await.insert(post_cache_key, cache_entry);

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
#[instrument(name = "create_comment", skip(session), fields(post_id = %comment_data.post_id, author = %comment_data.author))]
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
    
    let prepared = match session.prepare("INSERT INTO comments (id, post_id, content, author, created_at) VALUES (?, ?, ?, ?, ?)").await {
        Ok(p) => p,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Use timestamp_millis directly for ScyllaDB BIGINT
    DB_REQUEST_COUNTER.get().unwrap().inc();
    let result = session
        .execute(
            &prepared,
            (comment.id, comment.post_id, &comment.content, comment.created_at.timestamp_millis(), &comment.author),
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

/// Get comments by post with pagination
///
/// Returns paginated comments for a specific post using ScyllaDB native pagination
#[utoipa::path(
    get,
    path = "/posts/{post_id}/comments",
    params(
        ("post_id" = uuid::Uuid, Path, description = "Post ID"),
        ("page" = Option<u32>, Query, description = "Page number (starts at 1)", example = 1),
        ("limit" = Option<u32>, Query, description = "Number of items per page", example = 10)
    ),
    responses(
        (status = 200, description = "Paginated comments retrieved successfully", body = PaginatedResponse<Comment>),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(name = "get_comments_by_post", skip(session), fields(post_id = %path))]
#[get("/posts/{post_id}/comments")]
pub async fn get_comments_by_post(
    session: web::Data<Arc<Session>>,
    path: web::Path<Uuid>,
    pagination: Query<PaginationParams>,
) -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    let start = Instant::now();
    REQUEST_COUNTER.get().unwrap().inc();
    
    let post_id = path.into_inner();
    let page = pagination.page.max(1); // Ensure page >= 1
    let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

    info!("Fetching comments for post {} (page: {}, limit: {})", post_id, page, limit);

    // Prepare statement with page size for efficient pagination
    let mut prepared = match session.prepare("SELECT id, post_id, content, author, created_at FROM comments WHERE post_id = ? ALLOW FILTERING").await {
        Ok(stmt) => stmt,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
        }
    };
    
    // Set page size for efficient pagination
    prepared.set_page_size(limit as i32);

    DB_REQUEST_COUNTER.get().unwrap().inc();
    
    // Use execute_iter for paginated results
    let row_iterator = match session.execute_iter(prepared, (post_id,)).await {
        Ok(iterator) => iterator,
        Err(e) => {
            let duration = start.elapsed().as_secs_f64();
            HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
            return HttpResponse::InternalServerError().body(format!("Error executing query: {}", e));
        }
    };

    let mut comments = Vec::new();
    let mut total_fetched = 0u32;

    // Skip to the requested page
    let skip_count = (page - 1) * limit;
    let mut skipped = 0u32;

    // Convert iterator to stream and iterate through pages
    let mut rows_stream = row_iterator.into_typed::<(uuid::Uuid, uuid::Uuid, String, String, i64)>();
    
    while let Some(next_row_res) = rows_stream.next().await {
        match next_row_res {
            Ok((id, post_id, content, author, created_at_millis)) => {
                // Skip rows until we reach the desired page
                if skipped < skip_count {
                    skipped += 1;
                    continue;
                }
                
                // Stop if we have enough items for this page
                if total_fetched >= limit {
                    break;
                }

                // Convert timestamp
                let created_at = match Utc.timestamp_millis_opt(created_at_millis).single() {
                    Some(dt) => dt,
                    None => {
                        warn!("Invalid timestamp for comment {}: {}", id, created_at_millis);
                        continue;
                    }
                };

                comments.push(Comment {
                    id,
                    post_id,
                    content,
                    author,
                    created_at,
                });

                total_fetched += 1;
            },
            Err(e) => {
                error!("Error reading row: {}", e);
                let duration = start.elapsed().as_secs_f64();
                HTTP_REQUEST_DURATION.get().unwrap().observe(duration);
                return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
            }
        }
    }

    // Sort comments by created_at in ascending order (oldest first)
    comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let duration = start.elapsed();
    HTTP_REQUEST_DURATION.get().unwrap().observe(duration.as_secs_f64());

    // For pagination metadata, we'll estimate total pages
    // In a production system, you might want to maintain a separate count
    let has_more = total_fetched == limit; // If we got a full page, there might be more
    
    let meta = PaginationMeta {
        page,
        limit,
        total: None, // We don't have exact total count without additional query
        total_pages: if has_more { None } else { Some(page) }, // If no more data, current page is last
    };

    let response = PaginatedResponse {
        meta,
        data: comments,
    };

    info!("Successfully fetched {} comments for post {} (page: {}, limit: {}, duration: {}ms)", response.data.len(), post_id, page, limit, duration.as_millis());
    HttpResponse::Ok()
        .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
        .append_header(("X-Has-More", has_more.to_string()))
        .json(response)
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
#[instrument(name = "slow_endpoint")]
pub async fn slow_endpoint() -> impl Responder {
    init_metrics(); // Ensure metrics are initialized
    REQUEST_COUNTER.get().unwrap().inc();
    
    warn!("Slow endpoint called - simulating 600ms delay");
    // Simulate slow processing
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
    
    info!("Slow endpoint completed");
    HttpResponse::Ok().body("This endpoint is intentionally slow")
}


