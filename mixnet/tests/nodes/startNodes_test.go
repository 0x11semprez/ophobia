package nodes

import (
	"context"
	"mixnet/internal/models"
	"mixnet/internal/nodes"
	"mixnet/internal/route"
	"sync/atomic"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestStartAllNodes(t *testing.T) {
	const (
		layers    = 3
		perLayer  = 2
		providers = 2
		clients   = 4
	)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	var directory atomic.Pointer[route.Directory]

	mixNetwork, err := nodes.LaunchMixNodes(ctx, nodes.MixConfig{
		Host:      "127.0.0.1",
		BasePort:  39100,
		Layers:    layers,
		PerLayer:  perLayer,
		MeanDelay: 10 * time.Millisecond,
		LoopRate:  1,
		Handle:    nodes.RouteMixnode(&directory),
	})
	require.NoError(t, err)

	providerNetwork, err := nodes.LaunchProviders(ctx, nodes.ProviderConfig{
		Host:      "127.0.0.1",
		BasePort:  39000,
		Count:     providers,
		MeanDelay: 10 * time.Millisecond,
		LoopRate:  1,
		PullSize:  4,
		Handle:    nodes.RouteProvider(&directory),
	})
	require.NoError(t, err)

	dir := route.NewDirectory(mixNetwork.Mixnodes, providerNetwork.Providers)
	directory.Store(dir)

	running := make([]*nodes.RunningClient, 0, clients)
	for i := range clients {
		client, err := nodes.LaunchClient(ctx, nodes.ClientConfig{
			Host:        "127.0.0.1",
			Provider:    providerNetwork.Providers[i%providers],
			Directory:   dir,
			MeanDelay:   10 * time.Millisecond,
			PayloadRate: 1,
			LoopRate:    1,
			DropRate:    1,
		})
		require.NoError(t, err)
		running = append(running, client)
	}

	ids := make(map[models.NodeID]bool)

	assert.Len(t, mixNetwork.Mixnodes, layers*perLayer)
	for i, mixnode := range mixNetwork.Mixnodes {
		assert.Equal(t, i/perLayer, mixnode.Layer)
		assert.NotEmpty(t, mixnode.Address)
		assert.NotNil(t, mixnode.PublicKey)
		assert.False(t, ids[mixnode.ID], "duplicate node ID")
		ids[mixnode.ID] = true
	}

	assert.Len(t, providerNetwork.Providers, providers)
	for _, provider := range providerNetwork.Providers {
		assert.NotEmpty(t, provider.Address)
		assert.NotNil(t, provider.PublicKey)
		assert.NotNil(t, provider.Inbox(provider.ID))
		assert.False(t, ids[provider.ID], "duplicate node ID")
		ids[provider.ID] = true
	}

	assert.Len(t, dir.Layers, layers)
	assert.Len(t, dir.Providers, providers)

	for i, client := range running {
		provider := providerNetwork.Providers[i%providers]
		assert.Equal(t, provider.NodeInfo, client.Client.Provider)
		assert.NotNil(t, provider.Inbox(client.Client.ID))
		assert.False(t, ids[client.Client.ID], "duplicate node ID")
		ids[client.Client.ID] = true
	}

	cancel()
	for _, client := range running {
		client.Wait()
	}
	mixNetwork.Wait()
	providerNetwork.Wait()
}
