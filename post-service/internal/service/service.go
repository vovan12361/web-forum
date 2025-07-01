package service

import (
	"context"

	"github.com/gfdmit/web-forum/post-service/internal/repository"
)

type Service struct {
	repo repository.Repository
}

func New(repo repository.Repository) *Service {
	return &Service{repo: repo}
}

func (svc *Service) GetBoard(p context.Context, id string) (interface{}, error) {
	return svc.repo.GetBoard(p, id)
}

func (svc *Service) GetBoards(p context.Context, includeDeleted bool) (interface{}, error) {
	return svc.repo.GetBoards(p, includeDeleted)
}

func (svc *Service) GetPost(p context.Context, id string) (interface{}, error) {
	return svc.repo.GetPost(p, id)
}

func (svc *Service) GetPosts(p context.Context, boardID string, includeDeleted bool, limit int, offset int) (interface{}, error) {
	return svc.repo.GetPosts(p, boardID, includeDeleted, limit, offset)
}

func (svc *Service) GetComment(p context.Context, id string) (interface{}, error) {
	return svc.repo.GetComment(p, id)
}

func (svc *Service) GetComments(p context.Context, postID string, includeDeleted bool, limit int, offset int) (interface{}, error) {
	return svc.repo.GetComments(p, postID, includeDeleted, limit, offset)
}

func (svc *Service) CreateBoard(p context.Context, name string, description string) (interface{}, error) {
	return svc.repo.CreateBoard(p, name, description)
}

func (svc *Service) DeleteBoard(p context.Context, id string) (interface{}, error) {
	return svc.repo.DeleteBoard(p, id)
}

func (svc *Service) RestoreBoard(p context.Context, id string) (interface{}, error) {
	return svc.repo.RestoreBoard(p, id)
}

func (svc *Service) CreatePost(p context.Context, boardId string, title string, text string, hashIp string) (interface{}, error) {
	return svc.repo.CreatePost(p, boardId, title, text, hashIp)
}

func (svc *Service) DeletePost(p context.Context, id string) (interface{}, error) {
	return svc.repo.DeletePost(p, id)
}

func (svc *Service) CreateComment(p context.Context, postID string, text string, hashIp string) (interface{}, error) {
	return svc.repo.CreateComment(p, postID, text, hashIp)
}

func (svc *Service) DeleteComment(p context.Context, id string) (interface{}, error) {
	return svc.repo.DeleteComment(p, id)
}
