package config

import (
	"fmt"
	"time"

	"github.com/ilyakaznacheev/cleanenv"
	"github.com/joho/godotenv"
)

type Config struct {
	Postgres
	HTTPServer
}

type Postgres struct {
	User       string        `env:"POSTGRES_USER" env-default:"postgres"`
	Pass       string        `env:"POSTGRES_PASSWORD" env-default:"postgres"`
	Host       string        `env:"POSTGRES_HOST" env-default:"localhost"`
	Port       string        `env:"POSTGRES_PORT" env-default:"8000"`
	DB         string        `env:"POSTGRES_DB" env-default:"posts"`
	Timeout    time.Duration `env:"POSTGRES_TIMEOUT" env-default:"5s"`
	Migrations string        `env:"POSTGRES_MIGRATIONS" env-default:"./migrations"`
}

type HTTPServer struct {
	BindAddress     string        `env:"BIND_ADDRESS" env-default:"localhost"`
	BindPort        string        `env:"BIND_PORT" env-default:"8000"`
	ShutdownTimeout time.Duration `env:"SHUTDOWN_TIMEOUT" env-default:"5s"`
	ReadTimeout     time.Duration `env:"READ_TIMEOUT" env-default:"5s"`
	WriteTimeout    time.Duration `env:"WRITE_TIMEOUT" env-default:"5s"`
}

func New(env string) (*Config, error) {
	conf := &Config{}

	if err := godotenv.Overload(env); err != nil {
		return nil, fmt.Errorf("godotenv.Overload: %v", err)
	}

	if err := cleanenv.ReadEnv(conf); err != nil {
		return nil, fmt.Errorf("cleanenv.Readenv: %v", err)
	}

	return conf, nil
}
