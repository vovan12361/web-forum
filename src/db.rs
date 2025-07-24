use scylla::Session;

pub async fn init_db(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    // Create keyspace with optimized settings
    session
        .query(
            "CREATE KEYSPACE IF NOT EXISTS posts WITH REPLICATION = {
                'class': 'SimpleStrategy',
                'replication_factor': 1
            }",
            &[],
        ).await?;

    // Set keyspace
    session.use_keyspace("posts", false).await?;

    // Create boards table with optimizations
    session.query("
        CREATE TABLE IF NOT EXISTS boards (
            id UUID PRIMARY KEY,
            name TEXT,
            description TEXT,
            created_at BIGINT
        ) WITH compaction = {'class': 'LeveledCompactionStrategy'}
        AND compression = {'sstable_compression': 'LZ4Compressor'}
        AND gc_grace_seconds = 86400
    ", &[]).await?;

    // Add index on name for faster searches
    session.query(
        "CREATE INDEX IF NOT EXISTS boards_name_idx ON boards (name)", &[]
    ).await?;

    // Create posts table with optimizations
    session.query("
        CREATE TABLE IF NOT EXISTS posts (
            id UUID PRIMARY KEY,
            board_id UUID,
            title TEXT,
            content TEXT,
            created_at BIGINT,
            updated_at BIGINT,
            author TEXT
        ) WITH compaction = {'class': 'LeveledCompactionStrategy'}
        AND compression = {'sstable_compression': 'LZ4Compressor'}
        AND gc_grace_seconds = 86400
    ", &[]).await?;

    // Add index on board_id for faster board-specific queries
    session.query(
        "CREATE INDEX IF NOT EXISTS posts_board_idx ON posts (board_id)", &[]
    ).await?;

    // Create comments table with optimizations
    session.query("
        CREATE TABLE IF NOT EXISTS comments (
            id UUID PRIMARY KEY,
            post_id UUID,
            content TEXT,
            created_at BIGINT,
            author TEXT
        ) WITH compaction = {'class': 'LeveledCompactionStrategy'}
        AND compression = {'sstable_compression': 'LZ4Compressor'}
        AND gc_grace_seconds = 86400
    ", &[]).await?;

    // Add index on post_id for faster post-specific queries
    session.query(
        "CREATE INDEX IF NOT EXISTS comments_post_idx ON comments (post_id)", &[]
    ).await?;

    // Add index on author for faster author-specific queries
    session.query(
        "CREATE INDEX IF NOT EXISTS posts_author_idx ON posts (author)", &[]
    ).await?;

    session.query(
        "CREATE INDEX IF NOT EXISTS comments_author_idx ON comments (author)", &[]
    ).await?;

    // Add index on created_at for better time-based queries
    session.query(
        "CREATE INDEX IF NOT EXISTS posts_created_at_idx ON posts (created_at)", &[]
    ).await?;

    session.query(
        "CREATE INDEX IF NOT EXISTS comments_created_at_idx ON comments (created_at)", &[]
    ).await?;

    println!("Database initialized successfully with optimized indexes");
    Ok(())
}
