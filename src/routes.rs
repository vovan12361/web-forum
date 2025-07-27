    use actix_web::{get, post, web, HttpResponse, Responder, web::Query};
    use scylla::{Session, prepared_statement::PreparedStatement};
    use futures::stream::StreamExt;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use std::time::{Instant, Duration};
    use std::sync::Arc;
    use prometheus::{IntCounterVec, Histogram, Gauge, Counter};
    use std::sync::OnceLock;
    use tracing::{info, warn, error, debug, instrument};
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use serde_json;
    use crate::models::{
        Board, CreateBoardRequest, 
        Post, CreatePostRequest, 
        Comment, CreateCommentRequest,
        HealthResponse, PaginationParams, PaginatedResponse, PaginationMeta
    };

    // Wrapper types for different metric counters to avoid injection conflicts
    #[derive(Clone)]
    pub struct DbCounter(pub IntCounterVec);

    #[derive(Clone)]
    pub struct CacheCounter(pub IntCounterVec);

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

    /// Helper function to record database operation metrics
    fn record_db_operation(
        db_counter: &web::Data<DbCounter>,
        operation: &str,
        table: &str,
        success: bool,
    ) {
        let status = if success { "success" } else { "error" };
        db_counter.0.with_label_values(&[operation, table, status]).inc();
    }

    /// Helper function to record cache metrics
    fn record_cache_metric(cache_counter: &web::Data<CacheCounter>, cache_type: &str, result: &str) {
        cache_counter.0.with_label_values(&[cache_type, result]).inc();
    }

    /// Update memory usage metric
    fn update_memory_usage(memory_gauge: &web::Data<Gauge>) {
        // Get memory usage from /proc/self/status
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            memory_gauge.set(kb * 1024.0); // Convert KB to bytes
                            break;
                        }
                    }
                }
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
            get_post_by_id: session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE id = ?  ").await?,
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
    pub async fn health_check(
        memory_gauge: web::Data<Gauge>
    ) -> impl Responder {
        debug!("Health check requested");
        update_memory_usage(&memory_gauge);
        
        let response = HealthResponse {
            status: "OK".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        };
        
        info!("Health check successful");
        HttpResponse::Ok().json(response)
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
    #[instrument(name = "create_board", skip(session, db_counter), fields(board_name = %board_data.name))]
    pub async fn create_board(
        session: web::Data<Arc<Session>>,
        board_data: web::Json<CreateBoardRequest>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        let start = Instant::now();

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
        
        let _duration = start.elapsed();

        match result {
            Ok(_) => {
                info!("Board created successfully: {}", board.name);
                record_db_operation(&db_counter, "insert", "boards", true);
                HttpResponse::Created().json(board)
            },
            Err(e) => {
                error!("Error creating board: {}", e);
                record_db_operation(&db_counter, "insert", "boards", false);
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
    #[instrument(name = "get_boards", skip(session, db_counter))]
    pub async fn get_boards(
        session: web::Data<Arc<Session>>,
        pagination: Query<PaginationParams>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        let page = pagination.page.max(1); // Ensure page >= 1
        let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

        info!("Fetching boards (page: {}, limit: {})", page, limit);
        let start = Instant::now();

        // Prepare statement with page size
        let mut prepared = match session.prepare("SELECT id, name, description, created_at FROM boards").await {
            Ok(stmt) => stmt,
            Err(e) => {
                record_db_operation(&db_counter, "select", "boards", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        // Set page size for efficient pagination
        prepared.set_page_size(limit as i32);

        let _db_start = Instant::now();
        
        // Use execute_iter for paginated results
        let row_iterator = match session.execute_iter(prepared, &[]).await {
            Ok(iterator) => iterator,
            Err(e) => {
                record_db_operation(&db_counter, "select", "boards", false);
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
                    record_db_operation(&db_counter, "select", "boards", false);
                    return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
                }
            }
        }

        let duration = start.elapsed();
        record_db_operation(&db_counter, "select", "boards", true);

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
    #[instrument(name = "get_board", skip(session, db_counter, cache_counter), fields(board_id = %path))]
    pub async fn get_board(
        session: web::Data<Arc<Session>>,
        path: web::Path<Uuid>,
        db_counter: web::Data<DbCounter>,
        cache_counter: web::Data<CacheCounter>,
    ) -> impl Responder {
        let start = Instant::now();
        
        let board_id = path.into_inner();
        info!("Fetching board with ID: {}", board_id);
            
        // Check cache first
        let board_cache_key = board_id.to_string();
        if let Some(boards_cache) = BOARDS_CACHE.get() {
            if let Some(cached_board) = boards_cache.read().await.get(&board_cache_key) {
                if !cached_board.is_expired() {
                    info!("Cache hit for board ID: {}", board_id);
                    record_cache_metric(&cache_counter, "boards", "hit");
                    return HttpResponse::Ok().json(cached_board.get_data());
                } else {
                    info!("Cache expired for board ID: {}, fetching fresh data", board_id);
                    record_cache_metric(&cache_counter, "boards", "expired");
                }
            } else {
                info!("No cache entry for board ID: {}, fetching data", board_id);
                record_cache_metric(&cache_counter, "boards", "miss");
            }
        } else {
            warn!("Boards cache not initialized, fetching data from database");
            record_cache_metric(&cache_counter, "boards", "miss");
        }
        
        // Use prepared statement for better performance
        let result = if let Some(stmt) = GET_BOARD_STMT.get() {
            session.execute(stmt, (board_id,)).await
        } else {
            // Fallback to regular query if prepared statement not ready
            warn!("Prepared statement not available, using regular query");
            session.query("SELECT id, name, description, created_at FROM boards WHERE id = ?", (board_id,)).await
        };
        
        let _db_duration = start.elapsed();
        
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
                        if let Some(boards_cache) = BOARDS_CACHE.get() {
                            boards_cache.write().await.insert(board_cache_key, cache_entry);
                        }

                        record_db_operation(&db_counter, "select", "boards", true);
                        info!("Board found: {}", board.name);
                        return HttpResponse::Ok().json(board);
                    }
                }
                
                record_db_operation(&db_counter, "select", "boards", true);
                warn!("Board with id {} not found", board_id);
                HttpResponse::NotFound().body(format!("Board with id {} not found", board_id))
            }
            Err(e) => {
                record_db_operation(&db_counter, "select", "boards", false);
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
    #[instrument(name = "create_post", skip(session, db_counter), fields(board_id = %post_data.board_id, title = %post_data.title, author = %post_data.author))]
    pub async fn create_post(
        session: web::Data<Arc<Session>>,
        post_data: web::Json<CreatePostRequest>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        info!("Creating new post: '{}' by {} on board {}", post_data.title, post_data.author, post_data.board_id);
        
        let start = Instant::now();
        
        // First check if the board exists
        debug!("Checking if board exists: {}", post_data.board_id);
        let board_check = match session.prepare("SELECT id FROM boards WHERE id = ?").await {
            Ok(p) => {
                debug!("Board check query prepared successfully");
                p
            },
            Err(e) => {
                error!("Error preparing board check query: {}", e);
                record_db_operation(&db_counter, "select", "boards", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        let board_result = session.execute(&board_check, (post_data.board_id,)).await;
        
        match board_result {
            Ok(rows) => {
                if rows.rows.unwrap_or_default().is_empty() {
                    warn!("Board with id {} not found", post_data.board_id);
                    record_db_operation(&db_counter, "select", "boards", true);
                    return HttpResponse::BadRequest().body(format!("Board with id {} not found", post_data.board_id));
                } else {
                    debug!("Board exists, proceeding with post creation");
                    record_db_operation(&db_counter, "select", "boards", true);
                }
            },
            Err(e) => {
                error!("Error checking board existence: {}", e);
                record_db_operation(&db_counter, "select", "boards", false);
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
                record_db_operation(&db_counter, "insert", "posts", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        // Use timestamp_millis directly for ScyllaDB BIGINT
        debug!("Executing post insert query");
        let result = session
            .execute(
                &prepared,
                (post.id, post.board_id, &post.title, &post.content, &post.author, post.created_at.timestamp_millis(), post.updated_at.timestamp_millis()),
            )
            .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                info!("Post created successfully: '{}' (duration: {}ms)", post.title, duration.as_millis());
                record_db_operation(&db_counter, "insert", "posts", true);
                HttpResponse::Created()
                    .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
                    .json(post)
            },
            Err(e) => {
                error!("Error creating post: {}", e);
                record_db_operation(&db_counter, "insert", "posts", false);
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
    #[get("/boards/{board_id}/posts")]
    #[instrument(name = "get_posts_by_board", skip(session, db_counter), fields(board_id = %path))]
    pub async fn get_posts_by_board(
        session: web::Data<Arc<Session>>,
        path: web::Path<Uuid>,
        pagination: Query<PaginationParams>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        let board_id = path.into_inner();
        let page = pagination.page.max(1); // Ensure page >= 1
        let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

        info!("Fetching posts for board {} (page: {}, limit: {})", board_id, page, limit);
        let start = Instant::now();

        // Prepare statement with page size for efficient pagination
        let mut prepared = match session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE board_id = ? ALLOW FILTERING").await {
            Ok(stmt) => stmt,
            Err(e) => {
                record_db_operation(&db_counter, "select", "posts", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        // Set page size for efficient pagination
        prepared.set_page_size(limit as i32);
        
        // Use execute_iter for paginated results
        let row_iterator = match session.execute_iter(prepared, (board_id,)).await {
            Ok(iterator) => iterator,
            Err(e) => {
                record_db_operation(&db_counter, "select", "posts", false);
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
                    record_db_operation(&db_counter, "select", "posts", false);
                    return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
                }
            }
        }

        // Sort posts by created_at in descending order (newest first)
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let duration = start.elapsed();
        record_db_operation(&db_counter, "select", "posts", true);

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
    #[get("/posts/{post_id}")]
    #[instrument(name = "get_post", skip(session, db_counter, cache_counter), fields(post_id = %path))]
    pub async fn get_post(
        session: web::Data<Arc<Session>>,
        path: web::Path<Uuid>,
        db_counter: web::Data<DbCounter>,
        cache_counter: web::Data<CacheCounter>,
    ) -> impl Responder {
        let start = Instant::now();
        
        let post_id = path.into_inner();
        
        // Check cache first
        let post_cache_key = format!("post_{}", post_id);
        if let Some(posts_cache) = POSTS_CACHE.get() {
            if let Some(cached_post) = posts_cache.read().await.get(&post_cache_key) {
                if !cached_post.is_expired() {
                    info!("Cache hit for post ID: {}", post_id);
                    record_cache_metric(&cache_counter, "posts", "hit");
                    if let Some(post) = cached_post.get_data().first() {
                        return HttpResponse::Ok().json(post);
                    }
                } else {
                    info!("Cache expired for post ID: {}, fetching fresh data", post_id);
                    record_cache_metric(&cache_counter, "posts", "expired");
                }
            } else {
                info!("No cache entry for post ID: {}, fetching data", post_id);
                record_cache_metric(&cache_counter, "posts", "miss");
            }
        } else {
            warn!("Posts cache not initialized, fetching data from database");
            record_cache_metric(&cache_counter, "posts", "miss");
        }
        
        let prepared = match session.prepare("SELECT id, board_id, title, content, author, created_at, updated_at FROM posts WHERE id = ?").await {
            Ok(p) => p,
            Err(e) => {
                record_db_operation(&db_counter, "select", "posts", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        let result = session.execute(&prepared, (post_id,)).await;
        
        let duration = start.elapsed();
        
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
                            if let Some(posts_cache) = POSTS_CACHE.get() {
                                posts_cache.write().await.insert(post_cache_key, cache_entry);
                            }

                            record_db_operation(&db_counter, "select", "posts", true);
                            return HttpResponse::Ok()
                                .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
                                .json(post);
                        }
                    },
                    Err(_) => {}
                }
                
                record_db_operation(&db_counter, "select", "posts", true);
                HttpResponse::NotFound().body(format!("Post with id {} not found", post_id))
            }
            Err(e) => {
                record_db_operation(&db_counter, "select", "posts", false);
                HttpResponse::InternalServerError().body(format!("Error fetching post: {}", e))
            }
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
    #[instrument(name = "create_comment", skip(session, db_counter), fields(post_id = %comment_data.post_id, author = %comment_data.author))]
    pub async fn create_comment(
        session: web::Data<Arc<Session>>,
        comment_data: web::Json<CreateCommentRequest>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        info!("Creating comment for post_id: {}, author: {}", comment_data.post_id, comment_data.author);

        let start = Instant::now();
        
        // First check if the post exists
        let post_check = match session.prepare("SELECT id FROM posts WHERE id = ?").await {
            Ok(p) => p,
            Err(e) => {
                error!("Error preparing query: {}", e);
                record_db_operation(&db_counter, "select", "posts", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        let post_result = session.execute(&post_check, (comment_data.post_id,)).await;
        
        match post_result {
            Ok(rows) => {
                if rows.rows.unwrap_or_default().is_empty() {
                    error!("Post with id {} not found", comment_data.post_id);
                    record_db_operation(&db_counter, "select", "posts", true);
                    return HttpResponse::BadRequest().body(format!("Post with id {} not found", comment_data.post_id));
                } else {
                    record_db_operation(&db_counter, "select", "posts", true);
                }
            },
            Err(e) => {
                error!("Error checking post: {}", e);
                record_db_operation(&db_counter, "select", "posts", false);
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
                error!("Error preparing query: {}", e);
                record_db_operation(&db_counter, "insert", "comments", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        // Use timestamp_millis directly for ScyllaDB BIGINT
        let result = session
            .execute(
                &prepared,
                (comment.id, comment.post_id, &comment.content, &comment.author, comment.created_at.timestamp_millis()),
            )
            .await;

        let duration = start.elapsed();

        match result {
            Ok(_) => {
                record_db_operation(&db_counter, "insert", "comments", true);
                HttpResponse::Created()
                    .append_header(("X-Processing-Time-Ms", duration.as_millis().to_string()))
                    .json(comment)
            },
            Err(e) => {
                error!("Error creating comment: {}", e);
                record_db_operation(&db_counter, "insert", "comments", false);
                HttpResponse::InternalServerError().body(format!("Error creating comment: {}", e))
            }
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
    #[get("/posts/{post_id}/comments")]
    #[instrument(name = "get_comments_by_post", skip(session, db_counter), fields(post_id = %path))]
    pub async fn get_comments_by_post(
        session: web::Data<Arc<Session>>,
        path: web::Path<Uuid>,
        pagination: Query<PaginationParams>,
        db_counter: web::Data<DbCounter>,
    ) -> impl Responder {
        let start = Instant::now();
        
        let post_id = path.into_inner();
        let page = pagination.page.max(1); // Ensure page >= 1
        let limit = pagination.limit.max(1).min(100); // Ensure 1 <= limit <= 100

        info!("Fetching comments for post {} (page: {}, limit: {})", post_id, page, limit);

        // Prepare statement with page size for efficient pagination
        let mut prepared = match session.prepare("SELECT id, post_id, content, author, created_at FROM comments WHERE post_id = ? ALLOW FILTERING").await {
            Ok(stmt) => stmt,
            Err(e) => {
                record_db_operation(&db_counter, "select", "comments", false);
                return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
            }
        };
        
        // Set page size for efficient pagination
        prepared.set_page_size(limit as i32);
        
        // Use execute_iter for paginated results
        let row_iterator = match session.execute_iter(prepared, (post_id,)).await {
            Ok(iterator) => iterator,
            Err(e) => {
                record_db_operation(&db_counter, "select", "comments", false);
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
                    record_db_operation(&db_counter, "select", "comments", false);
                    return HttpResponse::InternalServerError().body(format!("Error reading row: {}", e));
                }
            }
        }

        // Sort comments by created_at in ascending order (oldest first)
        comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let duration = start.elapsed();
        record_db_operation(&db_counter, "select", "comments", true);

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

    /// Intentionally slow endpoint with CPU-intensive operations
    ///
    /// This endpoint is intentionally slow to demonstrate alerts and profiling
    #[utoipa::path(
        get,
        path = "/slow",
        responses(
            (status = 200, description = "Slow endpoint response with CPU profiling data")
        )
    )]
    #[get("/slow")]
    #[instrument(name = "slow_endpoint")]
    pub async fn slow_endpoint(
        cpu_counter: web::Data<Counter>,
        memory_gauge: web::Data<Gauge>,
        slow_duration: web::Data<Histogram>,
    ) -> impl Responder {
        cpu_counter.inc();
        
        let start = Instant::now();

        warn!("Slow endpoint called - starting CPU-intensive operations");
        update_memory_usage(&memory_gauge);
        
        // CPU-intensive computation in a blocking task
        let cpu_result = tokio::task::spawn_blocking(|| {
            info!("Starting CPU-intensive operations");
            
            // Multiple CPU-intensive operations
            let prime_result = heavy_cpu_computation(5000);
            let matrix_result = matrix_multiplication_result();
            let fib_result = fibonacci_iterative(35);
            
            info!("CPU-intensive operations completed");
            prime_result.wrapping_add(matrix_result).wrapping_add(fib_result)
        }).await.unwrap_or(0);
        
        // Still include some async delay
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        let duration = start.elapsed();
        slow_duration.observe(duration.as_secs_f64());
        update_memory_usage(&memory_gauge);

        info!("Slow endpoint completed with CPU result: {}, duration: {:?}", cpu_result, duration);
        HttpResponse::Ok().json(serde_json::json!({
            "message": "This endpoint is intentionally slow with CPU-intensive operations",
            "cpu_computation_result": cpu_result,
            "duration_ms": duration.as_millis(),
            "operations_performed": [
                "prime_number_calculation",
                "matrix_multiplication", 
                "fibonacci_calculation"
            ]
        }))
    }

    /// CPU-intensive mathematical computation for profiling
    /// This function will be easily visible in perf reports
    #[instrument(name = "heavy_cpu_computation")]
    fn heavy_cpu_computation(iterations: u64) -> u64 {
        info!("Starting heavy CPU computation with {} iterations", iterations);
        
        let mut result = 0u64;
        let mut temp_sum = 0u64;
        
        // Prime number calculation - CPU intensive
        for i in 2..iterations {
            if is_prime_slow(i) {
                result = result.wrapping_add(i);
                temp_sum = temp_sum.wrapping_add(i * i);
            }
        }
        
        // Additional mathematical operations
        let final_result = fibonacci_iterative(30) + matrix_multiplication_result() + temp_sum;
        
        info!("Heavy CPU computation completed, result: {}", final_result);
        final_result.wrapping_add(result)
    }

    /// Slow prime number check - intentionally inefficient for profiling
    #[instrument(name = "is_prime_slow")]
    fn is_prime_slow(n: u64) -> bool {
        if n < 2 {
            return false;
        }
        if n == 2 {
            return true;
        }
        if n % 2 == 0 {
            return false;
        }
        
        // Intentionally slow algorithm - checking all odd numbers up to sqrt(n)
        let limit = (n as f64).sqrt() as u64;
        for i in (3..=limit).step_by(2) {
            if n % i == 0 {
                return false;
            }
        }
        true
    }

    /// CPU-intensive Fibonacci calculation
    #[instrument(name = "fibonacci_iterative")]
    fn fibonacci_iterative(n: u32) -> u64 {
        if n == 0 {
            return 0;
        }
        if n == 1 {
            return 1;
        }
        
        let mut prev = 0u64;
        let mut curr = 1u64;
        
        for _ in 2..=n {
            let next = prev.wrapping_add(curr);
            prev = curr;
            curr = next;
        }
        
        curr
    }

    /// Simulated matrix multiplication for CPU load
    #[instrument(name = "matrix_multiplication_result")]
    fn matrix_multiplication_result() -> u64 {
        const SIZE: usize = 100;
        let mut matrix_a = vec![vec![1u32; SIZE]; SIZE];
        let mut matrix_b = vec![vec![2u32; SIZE]; SIZE];
        let mut result = vec![vec![0u64; SIZE]; SIZE];
        
        // Initialize matrices with some pattern
        for i in 0..SIZE {
            for j in 0..SIZE {
                matrix_a[i][j] = ((i + j) % 256) as u32;
                matrix_b[i][j] = ((i * j) % 256) as u32;
            }
        }
        
        // Matrix multiplication
        for i in 0..SIZE {
            for j in 0..SIZE {
                let mut sum = 0u64;
                for k in 0..SIZE {
                    sum = sum.wrapping_add((matrix_a[i][k] as u64) * (matrix_b[k][j] as u64));
                }
                result[i][j] = sum;
            }
        }
        
        // Return sum of diagonal elements
        let mut diagonal_sum = 0u64;
        for i in 0..SIZE {
            diagonal_sum = diagonal_sum.wrapping_add(result[i][i]);
        }
        
        diagonal_sum
    }