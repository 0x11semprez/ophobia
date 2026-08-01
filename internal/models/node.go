package models

import "github.com/oasisprotocol/curve25519-voi/primitives/x25519"

// NodeIDSize is the size of a node identifier: its x25519 public key.
const NodeIDSize = x25519.PointSize

// NodeID identifies a node or client in the network.
type NodeID [NodeIDSize]byte

// NodeInfo is the public directory entry of a node.
// Clients use it to build paths and derive per-hop shared secrets.
type NodeInfo struct {
	ID        NodeID
	Address   string // host:port, UDP
	PublicKey *x25519.PublicKey
}
