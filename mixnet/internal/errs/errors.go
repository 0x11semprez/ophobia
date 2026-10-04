// Package errs defines the mixnet sentinel errors, matched with errors.Is.
package errs

import "errors"

// Node errors.
var (
	ErrInvalidTopology      = errors.New("invalid topology")
	ErrInvalidProviderCount = errors.New("invalid provider count")
	ErrListen               = errors.New("listen failed")
	ErrMixnodeInit          = errors.New("mixnode init failed")
	ErrProviderInit         = errors.New("provider init failed")
	ErrClientInit           = errors.New("client init failed")
	ErrUnknownClient        = errors.New("unknown client")
	ErrMessageTooLarge      = errors.New("message too large")
	ErrOutboxFull           = errors.New("outbox full")
)

// Backend errors.
var (
	ErrEnvLoad     = errors.New("the .env didn't load")
	ErrServerStart = errors.New("server didn't start at all")
)
