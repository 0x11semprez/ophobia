package nodes

import (
	"context"
	"errors"
	"fmt"
	"log"
	"net"
	"strconv"
	"sync"

	"mixnet/internal/covertraffic"
	"mixnet/internal/errs"
	"mixnet/internal/models"
	"mixnet/internal/packets"
)

// outboxSize is how many real packets a client can queue before Send refuses more.
const outboxSize = 64

// ClientConfig describes one client and its Loopix sending rates.
type ClientConfig struct {
	Host        string // interface to bind, e.g. "127.0.0.1"
	Port        int    // 0 picks a free port
	Provider    *models.Provider
	PayloadRate float64 // lambda_P
	LoopRate    float64 // lambda_L
	DropRate    float64 // lambda_D

	// Handle processes each packet on the read loop, must not block; nil drops all.
	Handle func(c *models.Client, conn *net.UDPConn, p *models.SphinxPacket)
}

// RunningClient is a client sending its payload and cover streams.
type RunningClient struct {
	Client *models.Client
	outbox chan *models.SphinxPacket
	wg     sync.WaitGroup
}

// Send queues a real packet for the next payload slot, returning false if the outbox is full.
func (c *RunningClient) Send(p *models.SphinxPacket) bool {
	select {
	case c.outbox <- p:
		return true
	default:
		return false
	}
}

// Wait blocks until the client has stopped.
func (c *RunningClient) Wait() {
	c.wg.Wait()
}

// LaunchClient registers a client with its provider and runs its three Poisson streams until ctx is cancelled.
func LaunchClient(ctx context.Context, cfg ClientConfig) (*RunningClient, error) {
	if cfg.Provider == nil {
		return nil, fmt.Errorf("%w: no provider", errs.ErrClientInit)
	}
	providerAddress, err := net.ResolveUDPAddr("udp", cfg.Provider.Address)
	if err != nil {
		return nil, fmt.Errorf("%w: %w", errs.ErrClientInit, err)
	}

	address := net.JoinHostPort(cfg.Host, strconv.Itoa(cfg.Port))
	conn, err := listen(address)
	if err != nil {
		return nil, fmt.Errorf("%w: %s: %w", errs.ErrListen, address, err)
	}
	info, privateKey, err := newNodeInfo(conn)
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("%w: %w", errs.ErrClientInit, err)
	}

	client := &models.Client{
		NodeInfo:    info,
		PrivateKey:  privateKey,
		Provider:    cfg.Provider.NodeInfo,
		PayloadRate: cfg.PayloadRate,
		LoopRate:    cfg.LoopRate,
		DropRate:    cfg.DropRate,
	}
	cfg.Provider.Register(client.ID)

	running := &RunningClient{Client: client, outbox: make(chan *models.SphinxPacket, outboxSize)}
	name := "client"

	send := func(p *models.SphinxPacket) {
		_, err := conn.WriteToUDP(packets.Encode(p), providerAddress)
		if err != nil && !errors.Is(err, net.ErrClosed) {
			log.Printf("%s: write: %v", name, err)
		}
	}

	running.wg.Go(func() { covertraffic.PayloadStream(ctx, client.PayloadRate, running.outbox, send) })
	running.wg.Go(func() { covertraffic.LoopStream(ctx, client.LoopRate, send) })
	running.wg.Go(func() { covertraffic.DropStream(ctx, client.DropRate, send) })

	var handle func(*models.SphinxPacket)
	if cfg.Handle != nil {
		handle = func(p *models.SphinxPacket) { cfg.Handle(client, conn, p) }
	}
	running.wg.Go(func() { serve(ctx, name, conn, handle) })

	return running, nil
}
