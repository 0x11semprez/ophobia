package server

import (
	"fmt"
	"net/http"

	"mixnet/internal/backend/config"
	"mixnet/internal/errs"
)

func startServer(cfg *config.Config) error {
	err := http.ListenAndServe(cfg.Addr, cfg.Handler)
	if err != nil {
		return fmt.Errorf("%w: %w", errs.ErrServerStart, err)
	}

	return nil
}
