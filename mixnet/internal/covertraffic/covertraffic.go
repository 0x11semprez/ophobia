// Package covertraffic generates the Loopix fake traffic of senders and receivers.
package covertraffic

import (
	"context"
	"mixnet/internal/models"
	"mixnet/internal/packets"
	"mixnet/internal/poissonmix"
)

// Factory builds a fresh routed cover packet for each send.
type Factory func() *models.SphinxPacket

// PayloadStream sends a queued real packet every Exp(rate) interval, or a drop cover packet when outbox is empty.
func PayloadStream(ctx context.Context, rate float64, outbox <-chan *models.SphinxPacket, drop Factory, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() {
		select {
		case p := <-outbox:
			send(p)
		default:
			send(drop())
		}
	})
}

// LoopStream sends a loop cover packet, routed back to the sender, every Exp(rate) interval.
func LoopStream(ctx context.Context, rate float64, loop Factory, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() { send(loop()) })
}

// DropStream sends a drop cover packet, discarded by a random provider, every Exp(rate) interval.
func DropStream(ctx context.Context, rate float64, drop Factory, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() { send(drop()) })
}

// Pad appends random dummy messages until there are size, so every pull has the same size.
func Pad(messages [][models.PayloadSize]byte, size int) [][models.PayloadSize]byte {
	for len(messages) < size {
		messages = append(messages, packets.Random().Payload)
	}
	return messages
}
