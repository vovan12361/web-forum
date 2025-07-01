package v1

import (
	"github.com/gin-gonic/gin"
)

func New() (*gin.RouterGroup, error) {
	var (
		router = gin.Default()
	)

	api := router.Group("/api/v1")

	return api, nil
}
