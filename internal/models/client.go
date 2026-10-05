package models

import "github.com/oasisprotocol/curve25519-voi/primitives/x25519"

// Client is a user that sends and receives messages through its provider.
type Client struct {
	NodeInfo
	PrivateKey *x25519.PrivateKey

	// Provider is the entry and exit point the client is registered with.
	Provider NodeInfo

	// PayloadRate is lambda_P, the rate (packets/s) of the payload stream, padded with drop cover when idle.
	PayloadRate float64

	// LoopRate is lambda_L, the rate (packets/s) of client loop cover traffic.
	LoopRate float64

	// DropRate is lambda_D, the rate (packets/s) of client drop cover traffic.
	DropRate float64
}

// Payload framing until end-to-end encryption lands: PacketType, big-endian uint16 length, body, zero tail.
const (
	MessageHeaderSize = 3
	MaxMessageSize    = PayloadSize - MessageHeaderSize
)
