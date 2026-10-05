// Package poissonmix provides the Poisson timing of Loopix cover traffic.
package poissonmix

import (
	"context"
	"math/rand"
	"mixnet/internal/models"
	"mixnet/internal/packets"
	"time"
)

// ExponentialLaw samples a delay from Exp(lambda), in seconds with mean 1/lambda.
func ExponentialLaw(lambda float64) float64 {
	return rand.ExpFloat64() / lambda
}

// PoissonProcess waits one Exp(lambda) interval, then emits p to address via packets.Send.
func PoissonProcess(lambda float64, address string, p *models.SphinxPacket, size int) error {
	delay := ExponentialLaw(lambda)
	time.Sleep(time.Duration(delay * float64(time.Second)))
	return packets.Send(address, p, size)
}

// Run calls emit after each Exp(lambda) interval until ctx is cancelled; lambda <= 0 disables the stream.
func Run(ctx context.Context, lambda float64, emit func()) {
	if lambda <= 0 {
		return
	}
	for {
		timer := time.NewTimer(time.Duration(ExponentialLaw(lambda) * float64(time.Second)))
		select {
		case <-ctx.Done():
			timer.Stop()
			return
		case <-timer.C:
			emit()
		}
	}
}
