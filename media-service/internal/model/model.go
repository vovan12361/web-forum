package model

import (
	"net/url"

	"github.com/minio/minio-go/v7"
)

type MediaMeta struct {
	Info minio.UploadInfo
	Url  *url.URL
}
