package models

import (
	"sync"
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
	mu      sync.RWMutex
}

// Register gives client an empty inbox, keeping any existing one.
func (p *Provider) Register(client NodeID) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if _, ok := p.Inboxes[client]; !ok {
		p.Inboxes[client] = &Inbox{}
	}
}

// Inbox returns the inbox of client, or nil if the client is not registered.
func (p *Provider) Inbox(client NodeID) *Inbox {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.Inboxes[client]
}

// Inbox holds messages delivered to one client, still end-to-end encrypted.
type Inbox struct {
	Messages [][PayloadSize]byte
	mu       sync.Mutex
}

// Push stores one message at the end of the inbox.
func (i *Inbox) Push(message [PayloadSize]byte) {
	i.mu.Lock()
	defer i.mu.Unlock()
	i.Messages = append(i.Messages, message)
}

// Pop removes and returns up to n of the oldest messages.
func (i *Inbox) Pop(n int) [][PayloadSize]byte {
	i.mu.Lock()
	defer i.mu.Unlock()
	n = min(n, len(i.Messages))
	popped := i.Messages[:n:n]
	i.Messages = i.Messages[n:]
	return popped
}
