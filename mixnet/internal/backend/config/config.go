package config

import (
	"fmt"
	"net/http"
	"os"

	"github.com/joho/godotenv"

	"mixnet/internal/errs"
)

type Config struct {
	Addr    string
	Handler http.Handler
}

func Load() error {
	return godotenv.Load("../.env")
}

func NewConfig() (*Config, error) {
	err := Load()
	if err != nil {
		return nil, fmt.Errorf("%w: %w", errs.ErrEnvLoad, err)
	}

	mux := http.NewServeMux()

	return &Config{
		Addr:    os.Getenv("PORT"),
		Handler: mux,
	}, nil
}
