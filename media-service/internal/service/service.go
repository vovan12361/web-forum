package service

import (
	"fmt"
	"mime/multipart"
	"net/http"

	"github.com/gfdmit/web-forum/post-service/internal/repository"
	"github.com/gin-gonic/gin"
)

type Service struct {
	repo repository.Repository
}

func New(repo repository.Repository) *Service {
	return &Service{repo: repo}
}

func (svc *Service) PostImage(c *gin.Context) {
	file, header, err := c.Request.FormFile("image")
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "No file uploaded"})
		return
	}
	defer file.Close()
	if err := validateFile(header); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	mediaMeta, err := svc.repo.PostImage(file, header)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Upload failed"})
		return
	}
	c.JSON(http.StatusOK, gin.H{
		"filename": mediaMeta.Info.Key,
		"url":      mediaMeta.Url.String(),
		"size":     mediaMeta.Info.Size,
	})
}

func validateFile(header *multipart.FileHeader) error {
	allowed := map[string]bool{
		"image/jpeg": true,
		"image/png":  true,
		"image/jpg":  true,
	}
	if !allowed[header.Header.Get("Content-Type")] {
		return fmt.Errorf("invalid file type")
	}
	return nil
}
