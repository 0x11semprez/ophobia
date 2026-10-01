package nodes

import (
	"context"
	"fmt"
	"net"
	"sync"
	"time"

	"mixnet/internal/errs"
	"mixnet/internal/models"
)

// ProviderConfig describes the local providers to launch.
type ProviderConfig struct {
	Host      string // interface to bind, e.g. "127.0.0.1"
	BasePort  int    // provider i listens on BasePort+i
	Count     int    // number of providers
	MeanDelay time.Duration
	LoopRate  float64
	PullSize  int

	// Handle processes each well-formed packet a provider receives:
	// forward client packets to layer 0, or store them in a client inbox.
	// Handle runs on the read loop, so it must not block.
	// A nil Handle drops every packet.
	Handle func(p *models.Provider, conn *net.UDPConn, packet *models.SphinxPacket)
}

// ProviderNetwork is a set of running providers.
type ProviderNetwork struct {
	Providers []*models.Provider
	wg        sync.WaitGroup
}

// Wait blocks until every provider has stopped.
func (n *ProviderNetwork) Wait() {
	n.wg.Wait()
}

// LaunchProviders creates Count providers, binds one UDP socket per provider,
// and serves each one in its own goroutine until ctx is cancelled.
func LaunchProviders(ctx context.Context, cfg ProviderConfig) (*ProviderNetwork, error) {
	if cfg.Count <= 0 {
		return nil, fmt.Errorf("%w: %d", errs.ErrInvalidProviderCount, cfg.Count)
	}

	conns, err := listenAll(cfg.Host, cfg.BasePort, cfg.Count)
	if err != nil {
		return nil, err
	}

	network := &ProviderNetwork{Providers: make([]*models.Provider, len(conns))}
	for i, conn := range conns {
		info, privateKey, err := newNodeInfo(conn)
		if err != nil {
			closeAll(conns)
			return nil, fmt.Errorf("%w: provider %d: %w", errs.ErrProviderInit, i, err)
		}
		provider := &models.Provider{
			NodeInfo:   info,
			PrivateKey: privateKey,
			MeanDelay:  cfg.MeanDelay,
			LoopRate:   cfg.LoopRate,
			PullSize:   cfg.PullSize,
			Inboxes:    make(map[models.NodeID]*models.Inbox),
		}
		// Give the provider its own inbox, keyed by its node ID. Its loop
		// cover traffic returns to it, and lands here, so it can tell the
		// network is carrying its fake packets.
		provider.Inboxes[provider.ID] = &models.Inbox{}
		network.Providers[i] = provider
	}

	for i, provider := range network.Providers {
		conn := conns[i]
		var handle func(*models.SphinxPacket)
		if cfg.Handle != nil {
			handle = func(p *models.SphinxPacket) { cfg.Handle(provider, conn, p) }
		}
		name := fmt.Sprintf("provider %d", i)
		network.wg.Go(func() { serve(ctx, name, conn, handle) })
	}

	return network, nil
}
