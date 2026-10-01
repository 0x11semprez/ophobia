// Package poissonmix provides the Poisson timing of Loopix cover traffic.
// Each node emits packets at exponentially distributed intervals, so its
// output is a Poisson process that a passive observer cannot distinguish
// from any other node's. This hides when a real message is actually sent.
package poissonmix

import (
	"math/rand"
	"time"
)

// ExponentialLaw samples a delay from Exp(lambda), in seconds with mean 1/lambda.
func ExponentialLaw(lambda float64) float64 {
	return rand.ExpFloat64() / lambda
}

// PoissonProcess waits one Exp(lambda) interval, then emits one cover packet via SendPackets.
func PoissonProcess(lambda float64) {
	delay := ExponentialLaw(lambda)
	time.Sleep(time.Duration(delay * float64(time.Second)))
	SendPackets()
}

// SendPackets emits one cover-traffic packet onto the network (not yet implemented).
func SendPackets() {
}
