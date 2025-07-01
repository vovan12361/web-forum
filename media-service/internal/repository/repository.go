package repository

import (
	"mime/multipart"

	"github.com/gfdmit/web-forum/post-service/internal/model"
)

type Repository interface {
	PostImage(file multipart.File, header *multipart.FileHeader) (*model.MediaMeta, error)
}
