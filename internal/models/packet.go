// Package models defines the data structures shared across the mixnet.
package models

// Sphinx packet sizes, in bytes. Every packet on the wire has the same size,
// so a passive observer cannot tell real, loop and drop packets apart.
const (
	GroupElementSize = 32 // alpha: x25519 point, re-blinded at each hop
	MacSize          = 16 // gamma: truncated HMAC-SHA256 over beta
	MaxHops          = 5  // ingress provider + 3 mix layers + egress provider
	FlagSize         = 1  // RoutingFlag
	DelaySize        = 8  // per-hop delay, encoded as uint64 nanoseconds

	// HopInfoSize is one decrypted routing command: flag, next hop, delay, next MAC.
	HopInfoSize     = FlagSize + NodeIDSize + DelaySize + MacSize
	RoutingInfoSize = MaxHops * HopInfoSize // beta
	PayloadSize     = 1024                  // delta

	HeaderSize = GroupElementSize + RoutingInfoSize + MacSize
	PacketSize = HeaderSize + PayloadSize
)

// PacketType tells the final recipient what to do with a packet.
// It is only readable after the last layer is removed, never on the wire.
type PacketType uint8

const (
	// PacketReal carries a user message to a recipient's provider inbox.
	PacketReal PacketType = iota
	// PacketLoop returns to its sender, to detect active (n-1) attacks.
	PacketLoop
	// PacketDrop is cover traffic, discarded by the recipient provider.
	PacketDrop
)

// RoutingFlag tells a node how to handle a packet after it peels a layer.
type RoutingFlag uint8

const (
	// FlagRelay: hold the packet for Delay, then forward it to NextHop.
	FlagRelay RoutingFlag = iota
	// FlagDeliver: this node is the egress provider; store in the client inbox.
	FlagDeliver
)
