package httpserver

import (
	"context"
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gfdmit/web-forum/post-service/config"
)

type Server struct {
	server          *http.Server
	shutDownTimeout time.Duration
}

func New(conf config.HTTPServer, handler http.Handler) *Server {
	srv := &http.Server{
		Handler:      handler,
		ReadTimeout:  conf.ReadTimeout,
		WriteTimeout: conf.WriteTimeout,
		Addr:         fmt.Sprintf("%v:%v", conf.BindAddress, conf.BindPort),
	}

	s := &Server{
		server:          srv,
		shutDownTimeout: conf.ShutdownTimeout,
	}
	return s
}

func (s *Server) Run(ctx context.Context) error {
	log.Println("[HTTPSERVER] listening on:", s.server.Addr)

	go func() {
		err := s.server.ListenAndServe()
		if !errors.Is(err, http.ErrServerClosed) {
			log.Println("[HTTPSERVER] http server error:", err)
		}
	}()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	<-sigChan

	log.Println("[SHUTDOWN] http server shutdown")

	ctx, cancel := context.WithTimeout(ctx, s.shutDownTimeout)
	defer cancel()

	return s.server.Shutdown(ctx)
}
