DROP INDEX IF EXISTS posts.comments_post_id_idx;
DROP INDEX IF EXISTS posts.posts_board_id_idx;

DROP TABLE IF EXISTS posts.comments;
DROP TABLE IF EXISTS posts.posts;
DROP TABLE IF EXISTS posts.boards;

DROP SCHEMA IF EXISTS posts CASCADE;