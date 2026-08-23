// Package errs defines the sentinel errors of the mixnet.
// Call sites wrap them with fmt.Errorf("%w", ...) to add context,
// so callers can match them with errors.Is.
package errs

import "errors"

// Node errors.
var (
	ErrInvalidTopology      = errors.New("invalid topology")
	ErrInvalidProviderCount = errors.New("invalid provider count")
	ErrListen               = errors.New("listen failed")
	ErrMixnodeInit          = errors.New("mixnode init failed")
	ErrProviderInit         = errors.New("provider init failed")
)
