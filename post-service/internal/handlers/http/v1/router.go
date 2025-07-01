package v1

import (
	"net/http"
	"time"

	gql "github.com/gfdmit/web-forum/post-service/internal/handlers/http/v1/graphql"
	"github.com/gfdmit/web-forum/post-service/internal/service"
	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
)

func New(svc *service.Service) (*gin.Engine, error) {
	var (
		router = gin.New()
	)

	router.Use(cors.New(cors.Config{
		AllowOrigins:     []string{"*"},
		AllowMethods:     []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowHeaders:     []string{"Accept", "Authorization", "Content-Type", "X-CSRF-Token"},
		ExposeHeaders:    []string{"Link"},
		AllowCredentials: false,
		MaxAge:           300 * time.Second,
	}))

	gqlHandler, err := gql.New(svc)
	if err != nil {
		return nil, err
	}

	apiGroup := router.Group("/api/v1")
	{
		apiGroup.Use(gin.Logger())

		apiGroup.Any("/graphql", gin.WrapH(gqlHandler))

		authGroup := apiGroup.Group("")
		{
			authGroup.GET("/ping", func(c *gin.Context) {
				c.Status(http.StatusOK)
			})
		}
	}

	return router, nil
}
