use scylla::Session;

pub async fn init_db(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    // Используем query вместо execute для DDL запросов
    session
        .query(
            "CREATE KEYSPACE IF NOT EXISTS posts WITH REPLICATION = { 'class' : 'SimpleStrategy', 'replication_factor' : 1 }",
            &[],
        ).await?;

    // Установим keyspace
    session.use_keyspace("posts", false).await?;

    // Важно: используем query вместо execute для всех DDL запросов
    session.query("
        CREATE TABLE IF NOT EXISTS boards (
            id UUID PRIMARY KEY,
            name TEXT,
            description TEXT,
            created_at TIMESTAMP
        )
    ", &[]).await?;

    session.query("
        CREATE TABLE IF NOT EXISTS posts (
            id UUID PRIMARY KEY,
            board_id UUID,
            title TEXT,
            content TEXT,
            created_at TIMESTAMP,
            author TEXT
        )
    ", &[]).await?;

    session.query("
        CREATE TABLE IF NOT EXISTS comments (
            id UUID PRIMARY KEY,
            post_id UUID,
            content TEXT,
            created_at TIMESTAMP,
            author TEXT
        )
    ", &[]).await?;

    Ok(())
} 