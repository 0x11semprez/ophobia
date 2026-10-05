// Package nodes runs the mixnet nodes.
package nodes

import (
	"context"
	"fmt"
	"mixnet/internal/errs"
	"mixnet/internal/models"
	"net"
	"sync"
	"time"
)

// MixConfig describes the local stratified topology to launch.
type MixConfig struct {
	Host      string // interface to bind, e.g. "127.0.0.1"
	BasePort  int    // mixnode i listens on BasePort+i
	Layers    int    // number of mix layers, 3 in Loopix
	PerLayer  int    // mixnodes per layer
	MeanDelay time.Duration
	LoopRate  float64

	// Handle processes each packet on the read loop, must not block; nil drops all.
	Handle func(m *models.Mixnode, conn *net.UDPConn, p *models.SphinxPacket)
}

// MixNetwork is a set of running mixnodes.
type MixNetwork struct {
	Mixnodes []*models.Mixnode
	wg       sync.WaitGroup
}

// Wait blocks until every mixnode has stopped.
func (n *MixNetwork) Wait() {
	n.wg.Wait()
}

// LaunchMixNodes binds one UDP socket per mixnode and serves each in its own goroutine until ctx is cancelled.
func LaunchMixNodes(ctx context.Context, cfg MixConfig) (*MixNetwork, error) {
	if cfg.Layers <= 0 || cfg.PerLayer <= 0 {
		return nil, fmt.Errorf("%w: %d layers x %d mixnodes", errs.ErrInvalidTopology, cfg.Layers, cfg.PerLayer)
	}

	conns, err := listenAll(cfg.Host, cfg.BasePort, cfg.Layers*cfg.PerLayer)
	if err != nil {
		return nil, err
	}

	network := &MixNetwork{Mixnodes: make([]*models.Mixnode, len(conns))}
	for i, conn := range conns {
		info, privateKey, err := newNodeInfo(conn)
		if err != nil {
			closeAll(conns)
			return nil, fmt.Errorf("%w: mixnode %d: %w", errs.ErrMixnodeInit, i, err)
		}
		network.Mixnodes[i] = &models.Mixnode{
			NodeInfo:   info,
			PrivateKey: privateKey,
			Layer:      i / cfg.PerLayer,
			MeanDelay:  cfg.MeanDelay,
			LoopRate:   cfg.LoopRate,
		}
	}

	for i, mixnode := range network.Mixnodes {
		conn := conns[i]
		var handle func(*models.SphinxPacket)
		if cfg.Handle != nil {
			handle = func(p *models.SphinxPacket) { cfg.Handle(mixnode, conn, p) }
		}
		name := fmt.Sprintf("mixnode layer=%d", mixnode.Layer)
		network.wg.Go(func() { serve(ctx, name, conn, handle) })
	}

	return network, nil
}
