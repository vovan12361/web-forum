-- Добавляем тестовые доски
INSERT INTO posts.boards (name, description)
VALUES 
    ('b', 'Бред'),
    ('news', 'Ньюсач')
ON CONFLICT (name) DO NOTHING;

-- Добавляем тестовые посты
INSERT INTO posts.posts (board_id, title, text, hash_ip)
VALUES
    (1, 'First post!', 'Hello everyone!', '127.0.0.1'),
    (2, 'New feature', 'Check out the update!', '192.168.1.1');
    (3, 'Third post!', 'Wazzap!', '192.168.0.0');

-- Добавляем комментарии
INSERT INTO posts.comments (post_id, text, hash_ip)
VALUES
    (1, 'Welcome!', 'user123'),
    (1, 'Great start!', 'anon456');