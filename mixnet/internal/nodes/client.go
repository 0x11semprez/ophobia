package nodes

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"log"
	"math/rand"
	"mixnet/internal/covertraffic"
	"mixnet/internal/errs"
	"mixnet/internal/models"
	"mixnet/internal/packets"
	"mixnet/internal/route"
	"net"
	"slices"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

// outboxSize is how many real packets a client can queue before Send refuses more.
const outboxSize = 64

// ClientConfig describes one client and its Loopix sending rates.
type ClientConfig struct {
	Host        string // interface to bind, e.g. "127.0.0.1"
	Port        int    // 0 picks a free port
	Provider    *models.Provider
	Directory   *route.Directory
	MeanDelay   time.Duration // 1/mu, mean of the per-hop delays the client picks
	PayloadRate float64       // lambda_P
	LoopRate    float64       // lambda_L
	DropRate    float64       // lambda_D

	// Handle processes each packet on the read loop, must not block; nil drops all.
	Handle func(c *models.Client, conn *net.UDPConn, p *models.SphinxPacket)
}

// RunningClient is a client sending its payload and cover streams.
type RunningClient struct {
	Client    *models.Client
	provider  *models.Provider
	directory *route.Directory
	meanDelay time.Duration
	outbox    chan *models.SphinxPacket
	loops     atomic.Uint64
	wg        sync.WaitGroup
}

// SendMessage routes message to recipient through recipientProvider and queues it for the next payload slot.
func (c *RunningClient) SendMessage(recipient models.NodeID, recipientProvider models.NodeInfo, message []byte) error {
	if len(message) > models.MaxMessageSize {
		return fmt.Errorf("%w: %d > %d bytes", errs.ErrMessageTooLarge, len(message), models.MaxMessageSize)
	}
	if !c.Send(c.packet(models.PacketReal, recipientProvider, recipient, message)) {
		return errs.ErrOutboxFull
	}
	return nil
}

// Receive pulls the client inbox and returns the real messages, discarding padding and counting loops.
func (c *RunningClient) Receive() ([][]byte, error) {
	payloads, err := Pull(c.provider, c.Client.ID)
	if err != nil {
		return nil, err
	}
	var messages [][]byte
	for _, payload := range payloads {
		kind, body, ok := unframe(payload)
		switch {
		case !ok:
		case kind == models.PacketReal:
			messages = append(messages, body)
		case kind == models.PacketLoop:
			c.loops.Add(1)
		}
	}
	return messages, nil
}

// Loops returns how many of the client's loop packets have come back so far.
func (c *RunningClient) Loops() uint64 {
	return c.loops.Load()
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
	if cfg.Directory == nil || len(cfg.Directory.Providers) == 0 {
		return nil, fmt.Errorf("%w: no directory", errs.ErrClientInit)
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

	running := &RunningClient{
		Client:    client,
		provider:  cfg.Provider,
		directory: cfg.Directory,
		meanDelay: cfg.MeanDelay,
		outbox:    make(chan *models.SphinxPacket, outboxSize),
	}
	name := "client"

	send := func(p *models.SphinxPacket) {
		_, err := conn.WriteToUDP(packets.Encode(p), providerAddress)
		if err != nil && !errors.Is(err, net.ErrClosed) {
			log.Printf("%s: write: %v", name, err)
		}
	}

	running.wg.Go(func() { covertraffic.PayloadStream(ctx, client.PayloadRate, running.outbox, running.drop, send) })
	running.wg.Go(func() { covertraffic.LoopStream(ctx, client.LoopRate, running.loop, send) })
	running.wg.Go(func() { covertraffic.DropStream(ctx, client.DropRate, running.drop, send) })

	var handle func(*models.SphinxPacket)
	if cfg.Handle != nil {
		handle = func(p *models.SphinxPacket) { cfg.Handle(client, conn, p) }
	}
	running.wg.Go(func() { serve(ctx, name, conn, handle) })

	return running, nil
}

// packet frames body and routes it through a fresh path to recipient at egress.
func (c *RunningClient) packet(kind models.PacketType, egress models.NodeInfo, recipient models.NodeID, body []byte) *models.SphinxPacket {
	commands := route.Commands(c.directory.Path(), egress, recipient, c.meanDelay)
	return route.Build(commands, frame(kind, body))
}

// loop builds a loop cover packet routed back to the client through its own provider.
func (c *RunningClient) loop() *models.SphinxPacket {
	return c.packet(models.PacketLoop, c.Client.Provider, c.Client.ID, nil)
}

// drop builds a drop cover packet discarded by a random provider.
func (c *RunningClient) drop() *models.SphinxPacket {
	egress := c.directory.Providers[rand.Intn(len(c.directory.Providers))]
	return c.packet(models.PacketDrop, egress, route.DropID, nil)
}

// frame writes kind, the body length and body into a zero-padded payload.
func frame(kind models.PacketType, body []byte) [models.PayloadSize]byte {
	var payload [models.PayloadSize]byte
	payload[0] = byte(kind)
	binary.BigEndian.PutUint16(payload[1:], uint16(len(body)))
	copy(payload[models.MessageHeaderSize:], body)
	return payload
}

// unframe parses a payload written by frame, rejecting random padding by its type, length and non-zero tail.
func unframe(payload [models.PayloadSize]byte) (models.PacketType, []byte, bool) {
	kind := models.PacketType(payload[0])
	n := int(binary.BigEndian.Uint16(payload[1:]))
	if kind > models.PacketDrop || n > models.MaxMessageSize {
		return 0, nil, false
	}
	end := models.MessageHeaderSize + n
	if slices.ContainsFunc(payload[end:], func(b byte) bool { return b != 0 }) {
		return 0, nil, false
	}
	return kind, slices.Clone(payload[models.MessageHeaderSize:end]), true
}
