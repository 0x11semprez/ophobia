package main

import (
	"context"
	"flag"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"mixnet/internal/nodes"
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
	flag.Parse()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	mixNetwork, err := nodes.LaunchMixNodes(ctx, nodes.MixConfig{
		Host:      *host,
		BasePort:  *mixPort,
		Layers:    *layers,
		PerLayer:  *perLayer,
		MeanDelay: *meanDelay,
		LoopRate:  *loopRate,
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
	})
	if err != nil {
		stop()
		mixNetwork.Wait()
		log.Fatal(err)
	}

	<-ctx.Done()
	log.Print("shutting down")
	mixNetwork.Wait()
	providerNetwork.Wait()
}
