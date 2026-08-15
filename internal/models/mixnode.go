package models

import (
	"time"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"
)

// Mixnode is a relay in one layer of the stratified topology.
// Each packet stays in the mixnode for the delay chosen by the sender,
// which breaks the timing link between input and output packets.
type Mixnode struct {
	NodeInfo
	PrivateKey *x25519.PrivateKey

	// Layer is the position in the stratified topology (0, 1, 2).
	// A mixnode only forwards to mixnodes of Layer+1, or to a provider.
	Layer int

	// MeanDelay is 1/mu, the public mean of the exponential per-hop delay.
	MeanDelay time.Duration

	// LoopRate is lambda_M, the rate (packets/s) of mix loop cover traffic.
	LoopRate float64
}
