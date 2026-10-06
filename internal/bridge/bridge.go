// Package bridge exposes mixnet clients over local TCP so other processes can use the mixnet.
package bridge

import (
	"context"
	"encoding/binary"
	"fmt"
	"io"
	"log"
	"net"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"mixnet/internal/errs"
	"mixnet/internal/models"
	"mixnet/internal/nodes"
)

// Every message is a type byte, a big-endian uint16 length and the payload.
const (
	msgHello byte = 0 // bridge to peer: own NodeID, sent on connect
	msgSend  byte = 1 // peer to bridge: recipient NodeID then the frame to send
	msgFrame byte = 2 // bridge to peer: one frame received through the mixnet
	msgPeers byte = 3 // peer to bridge: empty request; bridge to peer: the other NodeIDs
)

const headerSize = 3

// Bridge serves one TCP port per mixnet client.
type Bridge struct {
	clients   []*nodes.RunningClient
	byID      map[models.NodeID]*nodes.RunningClient
	listeners []net.Listener
	poll      time.Duration
	wg        sync.WaitGroup
}

// Launch listens on host:basePort+i for client i (any free port if basePort is 0) until ctx is cancelled.
func Launch(ctx context.Context, host string, basePort int, clients []*nodes.RunningClient, poll time.Duration) (*Bridge, error) {
	b := &Bridge{clients: clients, byID: make(map[models.NodeID]*nodes.RunningClient), poll: poll}
	for i, c := range clients {
		port := 0
		if basePort > 0 {
			port = basePort + i
		}
		address := net.JoinHostPort(host, strconv.Itoa(port))
		listener, err := net.Listen("tcp", address)
		if err != nil {
			b.closeListeners()
			return nil, fmt.Errorf("%w: %s: %w", errs.ErrListen, address, err)
		}
		b.listeners = append(b.listeners, listener)
		b.byID[c.Client.ID] = c
	}

	context.AfterFunc(ctx, b.closeListeners)
	for i, c := range clients {
		b.wg.Go(func() { b.accept(ctx, c, b.listeners[i]) })
	}
	return b, nil
}

// Addrs returns the TCP address serving each client, in client order.
func (b *Bridge) Addrs() []string {
	addrs := make([]string, len(b.listeners))
	for i, l := range b.listeners {
		addrs[i] = l.Addr().String()
	}
	return addrs
}

// Wait blocks until every listener and session has stopped.
func (b *Bridge) Wait() {
	b.wg.Wait()
}

func (b *Bridge) closeListeners() {
	for _, l := range b.listeners {
		l.Close()
	}
}

// accept serves one connection at a time, refusing others while one is attached.
func (b *Bridge) accept(ctx context.Context, client *nodes.RunningClient, listener net.Listener) {
	var busy atomic.Bool
	for {
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		if !busy.CompareAndSwap(false, true) {
			conn.Close()
			continue
		}
		b.wg.Go(func() {
			defer busy.Store(false)
			b.session(ctx, client, conn)
		})
	}
}

// writer serialises writes, as the pull loop and the request handler share one connection.
type writer struct {
	mu   sync.Mutex
	conn net.Conn
}

func (w *writer) write(kind byte, payload []byte) error {
	if len(payload) > 0xffff {
		return fmt.Errorf("payload of %d bytes does not fit one message", len(payload))
	}
	buf := make([]byte, headerSize+len(payload))
	buf[0] = kind
	binary.BigEndian.PutUint16(buf[1:], uint16(len(payload)))
	copy(buf[headerSize:], payload)
	w.mu.Lock()
	defer w.mu.Unlock()
	_, err := w.conn.Write(buf)
	return err
}

func readMessage(r io.Reader) (byte, []byte, error) {
	var header [headerSize]byte
	if _, err := io.ReadFull(r, header[:]); err != nil {
		return 0, nil, err
	}
	payload := make([]byte, binary.BigEndian.Uint16(header[1:]))
	if _, err := io.ReadFull(r, payload); err != nil {
		return 0, nil, err
	}
	return header[0], payload, nil
}

func (b *Bridge) session(ctx context.Context, client *nodes.RunningClient, conn net.Conn) {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	defer conn.Close()
	stop := context.AfterFunc(ctx, func() { conn.Close() })
	defer stop()

	w := &writer{conn: conn}
	if err := w.write(msgHello, client.Client.ID[:]); err != nil {
		return
	}
	b.wg.Go(func() { b.pull(ctx, client, w) })

	for {
		kind, payload, err := readMessage(conn)
		if err != nil {
			return
		}
		switch kind {
		case msgSend:
			b.send(client, payload)
		case msgPeers:
			if w.write(msgPeers, b.peerIDs(client)) != nil {
				return
			}
		}
	}
}

// pull hands the peer every message that reached the client's inbox.
func (b *Bridge) pull(ctx context.Context, client *nodes.RunningClient, w *writer) {
	ticker := time.NewTicker(b.poll)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
		messages, err := client.Receive()
		if err != nil {
			log.Printf("bridge: receive: %v", err)
			return
		}
		for _, m := range messages {
			if w.write(msgFrame, m) != nil {
				return
			}
		}
	}
}

// send queues a frame for a known recipient and drops it otherwise, as the mixnet would.
func (b *Bridge) send(client *nodes.RunningClient, payload []byte) {
	if len(payload) < models.NodeIDSize {
		return
	}
	to := models.NodeID(payload[:models.NodeIDSize])
	target, ok := b.byID[to]
	if !ok {
		log.Printf("bridge: send to unknown peer %x", to[:4])
		return
	}
	if err := client.SendMessage(to, target.Client.Provider, payload[models.NodeIDSize:]); err != nil {
		log.Printf("bridge: send: %v", err)
	}
}

func (b *Bridge) peerIDs(self *nodes.RunningClient) []byte {
	var ids []byte
	for _, c := range b.clients {
		if c != self {
			ids = append(ids, c.Client.ID[:]...)
		}
	}
	return ids
}
