package nodes

import (
	"errors"
	"log"
	"mixnet/internal/models"
	"mixnet/internal/packets"
	"mixnet/internal/route"
	"net"
	"sync/atomic"
	"time"
)

// Garbage (random cover bytes, tampered packets) peels into nonsense commands, so every
// failed check below drops silently: without Sphinx MACs a bad hop cannot be told from noise.

// RouteMixnode returns a mixnode Handle that relays packets along dir, dropping anything else.
func RouteMixnode(dir *atomic.Pointer[route.Directory]) func(*models.Mixnode, *net.UDPConn, *models.SphinxPacket) {
	return func(m *models.Mixnode, conn *net.UDPConn, p *models.SphinxPacket) {
		hop := route.Peel(p)
		if hop.Flag == models.FlagRelay {
			relay(dir.Load(), conn, hop, p)
		}
	}
}

// RouteProvider returns a provider Handle that relays packets along dir and delivers final hops to client inboxes.
func RouteProvider(dir *atomic.Pointer[route.Directory]) func(*models.Provider, *net.UDPConn, *models.SphinxPacket) {
	return func(provider *models.Provider, conn *net.UDPConn, p *models.SphinxPacket) {
		hop := route.Peel(p)
		switch hop.Flag {
		case models.FlagRelay:
			relay(dir.Load(), conn, hop, p)
		case models.FlagDeliver:
			deliver(provider, hop, p)
		}
	}
}

// relay forwards p to hop.NextHop after hop.Delay without blocking the read loop.
func relay(dir *route.Directory, conn *net.UDPConn, hop models.HopInfo, p *models.SphinxPacket) {
	if dir == nil {
		return
	}
	next, ok := dir.Lookup(hop.NextHop)
	if !ok {
		return
	}
	address, err := net.ResolveUDPAddr("udp", next.Address)
	if err != nil {
		log.Printf("relay: resolve %s: %v", next.Address, err)
		return
	}
	buf := packets.Encode(p)
	time.AfterFunc(max(hop.Delay, 0), func() {
		_, err := conn.WriteToUDP(buf, address)
		if err != nil && !errors.Is(err, net.ErrClosed) {
			log.Printf("relay: write %s: %v", address, err)
		}
	})
}

// deliver stores the payload in the recipient's inbox, discarding drop cover and unknown recipients.
func deliver(provider *models.Provider, hop models.HopInfo, p *models.SphinxPacket) {
	if hop.NextHop == route.DropID {
		return
	}
	inbox := provider.Inbox(hop.NextHop)
	if inbox == nil {
		return
	}
	inbox.Push(p.Payload)
}
