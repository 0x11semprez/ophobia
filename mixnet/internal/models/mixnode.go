package models

import (
	"time"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"
)

// Mixnode is a relay in one layer of the stratified topology.
type Mixnode struct {
	NodeInfo
	PrivateKey *x25519.PrivateKey

	// Layer is the position in the stratified topology (0, 1, 2).
	Layer int

	// MeanDelay is 1/mu, the public mean of the exponential per-hop delay.
	MeanDelay time.Duration

	// LoopRate is lambda_M, the rate (packets/s) of mix loop cover traffic.
	LoopRate float64
}
