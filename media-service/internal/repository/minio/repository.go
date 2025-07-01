package minio

import (
	"context"
	"fmt"
	"log"
	"mime/multipart"
	"path/filepath"
	"time"

	"github.com/google/uuid"
	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"

	"github.com/gfdmit/web-forum/post-service/config"
	"github.com/gfdmit/web-forum/post-service/internal/model"
)

type minioRepository struct {
	cli    *minio.Client
	bucket string
}

func New(conf config.MinIO) (*minioRepository, error) {
	client, err := minio.New(fmt.Sprintf("%s:%s", conf.Host, conf.Port), &minio.Options{
		Creds:  credentials.NewStaticV4(conf.User, conf.Pass, ""),
		Secure: false,
	})
	if err != nil {
		log.Fatalf("MinIO init error: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	exists, err := client.BucketExists(ctx, conf.Bucket)
	if err != nil || !exists {
		err = client.MakeBucket(ctx, conf.Bucket, minio.MakeBucketOptions{})
		if err != nil {
			log.Fatalf("Bucket creation error: %v", err)
		}
	}

	repo := &minioRepository{
		cli:    client,
		bucket: conf.Bucket,
	}
	return repo, nil
}

func (mr minioRepository) PostImage(file multipart.File, header *multipart.FileHeader) (*model.MediaMeta, error) {
	ext := filepath.Ext(header.Filename)
	objectName := fmt.Sprintf("%s%s", uuid.New().String(), ext)

	info, err := mr.cli.PutObject(
		context.Background(),
		mr.bucket,
		objectName,
		file,
		header.Size,
		minio.PutObjectOptions{ContentType: header.Header.Get("Content-Type")},
	)
	if err != nil {
		return nil, err
	}

	url, err := mr.cli.PresignedGetObject(
		context.Background(),
		mr.bucket,
		objectName,
		24*time.Hour,
		nil,
	)
	if err != nil {
		return nil, err
	}
	mediaMeta := &model.MediaMeta{
		Info: info,
		Url:  url,
	}
	return mediaMeta, nil
}
