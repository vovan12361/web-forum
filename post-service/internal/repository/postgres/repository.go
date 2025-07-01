package postgres

import (
	"context"
	"crypto/sha1"
	"database/sql"
	"encoding/base64"
	"errors"
	"fmt"
	"log"
	"strconv"

	"github.com/gfdmit/web-forum/post-service/config"
	"github.com/gfdmit/web-forum/post-service/internal/repository"
	"github.com/golang-migrate/migrate/v4"
	"github.com/golang-migrate/migrate/v4/database/postgres"

	_ "github.com/golang-migrate/migrate/v4/source/file"
	_ "github.com/lib/pq"
)

type postgresRepository struct {
	db *sql.DB
}

func New(conf config.Postgres) (*postgresRepository, error) {
	url := fmt.Sprintf(
		"postgresql://%v:%v@%v:%v/%v?sslmode=disable", conf.User, conf.Pass, conf.Host, conf.Port, conf.DB)

	db, err := sql.Open("postgres", url)
	if err != nil {
		return nil, fmt.Errorf("sql.Open: %v", err)
	}
	err = db.Ping()
	if err != nil {
		return nil, fmt.Errorf("db.Ping: %v", err)
	}
	driver, err := postgres.WithInstance(db, &postgres.Config{})
	if err != nil {
		return nil, fmt.Errorf("postgers.WithInstance: %v", err)
	}
	migrations := fmt.Sprintf("file://%v", conf.Migrations)
	m, err := migrate.NewWithDatabaseInstance(migrations, conf.DB, driver)
	if err != nil {
		return nil, fmt.Errorf("migrate.NewWithDatabaseInstance: %v", err)
	}
	log.Println("applying migrations...")
	if err := m.Up(); err != nil {
		if errors.Is(err, migrate.ErrNoChange) {
			log.Println("nothing to migrate")
		} else {
			return nil, fmt.Errorf("error when migrating: %v", err)
		}
	} else {
		log.Println("migrated successfully!")
	}

	return &postgresRepository{
		db: db,
	}, nil
}

func (pr postgresRepository) GetBoard(p context.Context, id string) (interface{}, error) {
	board := &repository.Board{}
	err := pr.db.QueryRow("SELECT * FROM posts.boards WHERE id = $1", id).Scan(
		&board.ID, &board.Name, &board.Description, &board.CreatedAt, &board.DeletedAt)
	if err != nil {
		return nil, err
	}
	return board, nil
}

func (pr postgresRepository) GetBoards(p context.Context, includeDeleted bool) (interface{}, error) {
	boards := []repository.Board{}
	var (
		rows *sql.Rows
		err  error
	)
	if includeDeleted {
		rows, err = pr.db.Query("SELECT * FROM posts.boards")
	} else {
		rows, err = pr.db.Query("SELECT * FROM posts.boards WHERE deleted_at IS NULL")
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		board := repository.Board{}
		err = rows.Scan(&board.ID, &board.Name, &board.Description, &board.CreatedAt, &board.DeletedAt)
		if err != nil {
			return nil, err
		}
		boards = append(boards, board)
	}
	return boards, nil
}

func (pr postgresRepository) GetPost(p context.Context, id string) (interface{}, error) {
	post := &repository.Post{}
	err := pr.db.QueryRow("SELECT * FROM posts.posts WHERE id = $1", id).Scan(
		&post.ID, &post.BoardID, &post.Title, &post.Text, &post.HashIP, &post.CreatedAt, &post.DeletedAt)
	if err != nil {
		return nil, err
	}
	return post, nil
}

func (pr postgresRepository) GetPosts(p context.Context, boardID string, includeDeleted bool, limit int, offset int) (interface{}, error) {
	posts := []repository.Post{}
	var (
		rows *sql.Rows
		err  error
	)
	if includeDeleted {
		rows, err = pr.db.Query("SELECT * FROM posts.posts WHERE board_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3", boardID, limit, offset)
	} else {
		rows, err = pr.db.Query("SELECT * FROM posts.posts WHERE deleted_at IS NULL AND board_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3", boardID, limit, offset)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		post := repository.Post{}
		err = rows.Scan(
			&post.ID, &post.BoardID, &post.Title, &post.Text, &post.HashIP, &post.CreatedAt, &post.DeletedAt)
		if err != nil {
			return nil, err
		}
		posts = append(posts, post)
	}
	return posts, nil
}

func (pr postgresRepository) GetComment(p context.Context, id string) (interface{}, error) {
	comment := &repository.Comment{}
	err := pr.db.QueryRow("SELECT * FROM posts.comments WHERE id = $1", id).Scan(
		&comment.ID, &comment.PostID, &comment.Text, &comment.HashIP, &comment.CreatedAt, &comment.DeletedAt)
	if err != nil {
		return nil, err
	}
	return comment, nil
}

func (pr postgresRepository) GetComments(p context.Context, postID string, includeDeleted bool, limit int, offset int) (interface{}, error) {
	comments := []repository.Comment{}
	var (
		rows *sql.Rows
		err  error
	)
	if includeDeleted {
		rows, err = pr.db.Query("SELECT * FROM posts.comments WHERE post_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3", postID, limit, offset)
	} else {
		rows, err = pr.db.Query("SELECT * FROM posts.comments WHERE deleted_at IS NULL AND post_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3", postID, limit, offset)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		comment := repository.Comment{}
		err = rows.Scan(
			&comment.ID, &comment.PostID, &comment.Text, &comment.HashIP, &comment.CreatedAt, &comment.DeletedAt)
		if err != nil {
			return nil, err
		}
		comments = append(comments, comment)
	}
	return comments, nil
}

func (pr postgresRepository) CreateBoard(p context.Context, name string, description string) (interface{}, error) {
	var (
		boardId int
	)
	err := pr.db.QueryRow("INSERT INTO posts.boards (name, description) VALUES($1, $2) RETURNING id", name, description).Scan(&boardId)
	if err != nil {
		return nil, err
	}
	return pr.GetBoard(p, strconv.Itoa(boardId))
}

func (pr postgresRepository) DeleteBoard(p context.Context, id string) (interface{}, error) {
	stmt, err := pr.db.Prepare("UPDATE posts.boards SET deleted_at = NOW() WHERE id = $1")
	if err != nil {
		return false, err
	}
	defer stmt.Close()

	_, err = stmt.Exec(id)
	if err != nil {
		return false, err
	}
	return true, nil
}

func (pr postgresRepository) RestoreBoard(p context.Context, id string) (interface{}, error) {
	stmt, err := pr.db.Prepare("UPDATE posts.boards SET deleted_at = NULL WHERE id = $1")
	if err != nil {
		return false, err
	}
	defer stmt.Close()

	_, err = stmt.Exec(id)
	if err != nil {
		return false, err
	}
	return true, nil
}

func (pr postgresRepository) CreatePost(p context.Context, boardId string, title string, text string, hashIp string) (interface{}, error) {
	var (
		postId int
	)
	hashIp = hashingIP(hashIp)
	err := pr.db.QueryRow("INSERT INTO posts.posts (board_id, title, text, hash_ip) VALUES($1, $2, $3, $4) RETURNING id", boardId, title, text, hashIp).Scan(&postId)
	if err != nil {
		return nil, err
	}
	return pr.GetPost(p, strconv.Itoa(postId))
}

func (pr postgresRepository) DeletePost(p context.Context, id string) (interface{}, error) {
	stmt, err := pr.db.Prepare("UPDATE posts.posts SET deleted_at = NOW() WHERE id = $1")
	if err != nil {
		return false, err
	}
	defer stmt.Close()

	_, err = stmt.Exec(id)
	if err != nil {
		return false, err
	}
	return true, nil
}

func (pr postgresRepository) CreateComment(p context.Context, postID string, text string, hashIp string) (interface{}, error) {
	var (
		commentId int
	)
	hashIp = hashingIP(hashIp)
	err := pr.db.QueryRow("INSERT INTO posts.comments (post_id, text, hash_ip) VALUES($1, $2, $3) RETURNING id", postID, text, hashIp).Scan(&commentId)
	if err != nil {
		return nil, err
	}
	return pr.GetComment(p, strconv.Itoa(commentId))
}

func (pr postgresRepository) DeleteComment(p context.Context, id string) (interface{}, error) {
	stmt, err := pr.db.Prepare("UPDATE posts.comments SET deleted_at = NOW() WHERE id = $1")
	if err != nil {
		return false, err
	}
	defer stmt.Close()

	_, err = stmt.Exec(id)
	if err != nil {
		return false, err
	}
	return true, nil
}

func hashingIP(hashIp string) string {
	hasher := sha1.New()
	hasher.Write([]byte(hashIp))
	hashIp = base64.URLEncoding.EncodeToString(hasher.Sum(nil))
	return hashIp
}
