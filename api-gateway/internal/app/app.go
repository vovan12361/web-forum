package app

import (
	"log"

	"github.com/gfdmit/web-forum/api-gateway/config"
	v1 "github.com/gfdmit/web-forum/api-gateway/internal/handlers/http/v1"
)

func Run(conf config.App) error {
	api, err := v1.New()
	if err != nil {
		return err
	}
	if err := api.Run(":8080"); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
	return nil
}
