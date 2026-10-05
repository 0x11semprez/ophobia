// Package covertraffic generates the Loopix fake traffic of senders and receivers.
package covertraffic

import (
	"context"

	"mixnet/internal/models"
	"mixnet/internal/packets"
	"mixnet/internal/poissonmix"
)

// PayloadStream sends a queued real packet every Exp(rate) interval, or a drop cover packet when outbox is empty.
func PayloadStream(ctx context.Context, rate float64, outbox <-chan *models.SphinxPacket, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() {
		select {
		case p := <-outbox:
			send(p)
		default:
			send(packets.Random())
		}
	})
}

// LoopStream sends a loop cover packet every Exp(rate) interval; random bytes until Sphinx routes it back to the sender.
func LoopStream(ctx context.Context, rate float64, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() { send(packets.Random()) })
}

// DropStream sends a drop cover packet every Exp(rate) interval; random bytes until Sphinx routes it to a provider.
func DropStream(ctx context.Context, rate float64, send func(*models.SphinxPacket)) {
	poissonmix.Run(ctx, rate, func() { send(packets.Random()) })
}

// Pad appends random dummy messages until there are size, so every pull has the same size.
func Pad(messages [][models.PayloadSize]byte, size int) [][models.PayloadSize]byte {
	for len(messages) < size {
		messages = append(messages, packets.Random().Payload)
	}
	return messages
}
