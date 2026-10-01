package models

import (
	"time"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"
)

// Provider is the entry and exit point of the mixnet for its clients.
type Provider struct {
	NodeInfo
	PrivateKey *x25519.PrivateKey

	// MeanDelay is 1/mu, the public mean of the exponential per-hop delay.
	MeanDelay time.Duration

	// LoopRate is lambda_M, the rate (packets/s) of provider loop cover traffic.
	LoopRate float64

	// PullSize is the fixed number of messages returned per pull.
	PullSize int

	// Inboxes maps each registered client to its stored messages.
	Inboxes map[NodeID]*Inbox
}

// Inbox holds messages delivered to one client, still end-to-end encrypted.
type Inbox struct {
	Messages [][PayloadSize]byte
}
