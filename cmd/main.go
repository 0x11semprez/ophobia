package main

import (
	"context"
	"flag"
	"log"
	"os"
	"os/signal"
	"sync/atomic"
	"syscall"
	"time"

	"mixnet/internal/nodes"
	"mixnet/internal/route"
)

func main() {
	host := flag.String("host", "127.0.0.1", "interface to bind")
	providerPort := flag.Int("provider-port", 9000, "first provider port")
	providers := flag.Int("providers", 2, "number of providers")
	mixPort := flag.Int("mix-port", 9100, "first mixnode port")
	layers := flag.Int("layers", 3, "number of mix layers")
	perLayer := flag.Int("per-layer", 3, "mixnodes per layer")
	meanDelay := flag.Duration("mean-delay", 100*time.Millisecond, "mean per-hop delay")
	loopRate := flag.Float64("loop-rate", 1, "loop cover traffic rate, packets/s")
	pullSize := flag.Int("pull-size", 10, "messages returned per pull")
	clients := flag.Int("clients", 4, "number of clients, spread over the providers")
	payloadRate := flag.Float64("payload-rate", 2, "client payload stream rate, packets/s")
	clientLoopRate := flag.Float64("client-loop-rate", 1, "client loop cover traffic rate, packets/s")
	dropRate := flag.Float64("drop-rate", 1, "client drop cover traffic rate, packets/s")
	flag.Parse()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	// Handlers start before the directory exists and drop packets until it is stored.
	var directory atomic.Pointer[route.Directory]

	mixNetwork, err := nodes.LaunchMixNodes(ctx, nodes.MixConfig{
		Host:      *host,
		BasePort:  *mixPort,
		Layers:    *layers,
		PerLayer:  *perLayer,
		MeanDelay: *meanDelay,
		LoopRate:  *loopRate,
		Handle:    nodes.RouteMixnode(&directory),
	})
	if err != nil {
		log.Fatal(err)
	}

	providerNetwork, err := nodes.LaunchProviders(ctx, nodes.ProviderConfig{
		Host:      *host,
		BasePort:  *providerPort,
		Count:     *providers,
		MeanDelay: *meanDelay,
		LoopRate:  *loopRate,
		PullSize:  *pullSize,
		Handle:    nodes.RouteProvider(&directory),
	})
	if err != nil {
		stop()
		mixNetwork.Wait()
		log.Fatal(err)
	}
	directory.Store(route.NewDirectory(mixNetwork.Mixnodes, providerNetwork.Providers))

	running := make([]*nodes.RunningClient, 0, *clients)
	for i := range *clients {
		client, err := nodes.LaunchClient(ctx, nodes.ClientConfig{
			Host:        *host,
			Provider:    providerNetwork.Providers[i%len(providerNetwork.Providers)],
			PayloadRate: *payloadRate,
			LoopRate:    *clientLoopRate,
			DropRate:    *dropRate,
		})
		if err != nil {
			log.Print(err)
			stop()
			break
		}
		running = append(running, client)
	}

	<-ctx.Done()
	log.Print("shutting down")
	for _, client := range running {
		client.Wait()
	}
	mixNetwork.Wait()
	providerNetwork.Wait()
}
