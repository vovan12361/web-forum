package repository

import "context"

type Repository interface {
	GetBoard(p context.Context, id string) (interface{}, error)
	GetBoards(p context.Context, includeDeleted bool) (interface{}, error)
	GetPost(p context.Context, id string) (interface{}, error)
	GetPosts(p context.Context, boardID string, includeDeleted bool, limit int, offset int) (interface{}, error)
	GetComment(p context.Context, id string) (interface{}, error)
	GetComments(p context.Context, postID string, includeDeleted bool, limit int, offset int) (interface{}, error)
	CreateBoard(p context.Context, name string, description string) (interface{}, error)
	DeleteBoard(p context.Context, id string) (interface{}, error)
	RestoreBoard(p context.Context, id string) (interface{}, error)
	CreatePost(p context.Context, boardId string, title string, text string, hashIp string) (interface{}, error)
	DeletePost(p context.Context, id string) (interface{}, error)
	CreateComment(p context.Context, postID string, text string, hashIp string) (interface{}, error)
	DeleteComment(p context.Context, id string) (interface{}, error)
}
