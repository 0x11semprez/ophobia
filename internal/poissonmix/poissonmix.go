// Package poissonmix provides the Poisson timing of Loopix cover traffic.
// Each node emits packets at exponentially distributed intervals, so its
// output is a Poisson process that a passive observer cannot distinguish
// from any other node's. This hides when a real message is actually sent.
package poissonmix

import (
	"math/rand"
	"time"
)

// ExponentialLaw samples a delay from the exponential distribution Exp(lambda).
// lambda is a rate in events per second, so the returned value is a wait in
// seconds with mean 1/lambda. This is the per-hop delay source of Poisson mixing.
func ExponentialLaw(lambda float64) float64 {
	return rand.ExpFloat64() / lambda
}

// PoissonProcess waits one exponentially distributed interval for rate lambda,
// then emits a single cover packet. Called in a loop, it drives a steady stream
// of traffic whose timing leaks nothing about the real messages inside it.
func PoissonProcess(lambda float64) {
	delay := ExponentialLaw(lambda)
	time.Sleep(time.Duration(delay * float64(time.Second)))
	SendPackets()
}

// SendPackets emits one cover-traffic packet onto the network.
//
// Not yet implemented: the body is intentionally empty for now. It will build a
// loop or drop Sphinx packet and write it to the node's first hop.
func SendPackets() {
}
