package app

import (
	"context"
	"fmt"

	"github.com/gfdmit/web-forum/post-service/config"
	v1 "github.com/gfdmit/web-forum/post-service/internal/handlers/http/v1"
	"github.com/gfdmit/web-forum/post-service/internal/httpserver"
	"github.com/gfdmit/web-forum/post-service/internal/repository/minio"
	"github.com/gfdmit/web-forum/post-service/internal/service"
)

func Run(conf config.Config) error {
	ctx := context.Background()
	repo, err := minio.New(conf.MinIO)

	if err != nil {
		return fmt.Errorf("error when setting up repository: %v", err)
	}

	service := service.New(repo)

	handler, err := v1.New(service)
	if err != nil {
		return fmt.Errorf("error when setting up handler: %v", err)
	}

	httpserver := httpserver.New(conf.HTTPServer, handler)

	return httpserver.Run(ctx)
}
