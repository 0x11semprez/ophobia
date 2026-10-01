package nodes

import (
	"context"
	"crypto/rand"
	"errors"
	"fmt"
	"log"
	"net"
	"strconv"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"

	"mixnet/internal/errs"
	"mixnet/internal/models"
)

// listenAll binds count consecutive UDP sockets from basePort, closing any it opened if one fails.
func listenAll(host string, basePort, count int) ([]*net.UDPConn, error) {
	conns := make([]*net.UDPConn, 0, count)
	for i := range count {
		address := net.JoinHostPort(host, strconv.Itoa(basePort+i))
		conn, err := listen(address)
		if err != nil {
			closeAll(conns)
			return nil, fmt.Errorf("%w: %s: %w", errs.ErrListen, address, err)
		}
		conns = append(conns, conn)
	}
	return conns, nil
}

func listen(address string) (*net.UDPConn, error) {
	udpAddress, err := net.ResolveUDPAddr("udp", address)
	if err != nil {
		return nil, err
	}
	return net.ListenUDP("udp", udpAddress)
}

// serve reads fixed-size packets until ctx is cancelled, dropping malformed datagrams and, with a nil handle, every packet.
func serve(ctx context.Context, name string, conn *net.UDPConn, handle func(*models.SphinxPacket)) {
	stop := context.AfterFunc(ctx, func() { conn.Close() })
	defer stop()
	defer conn.Close()

	log.Printf("%s listening on %s", name, conn.LocalAddr())

	// One extra byte detects oversized datagrams, which ReadFromUDP truncates.
	buf := make([]byte, models.PacketSize+1)
	for {
		n, _, err := conn.ReadFromUDP(buf)
		if err != nil {
			if !errors.Is(err, net.ErrClosed) {
				log.Printf("%s: read: %v", name, err)
			}
			return
		}
		if n != models.PacketSize || handle == nil {
			continue
		}
		handle(decodePacket(buf[:n]))
	}
}

// decodePacket splits a models.PacketSize buffer into the Sphinx fields.
func decodePacket(buf []byte) *models.SphinxPacket {
	var p models.SphinxPacket
	offset := copy(p.Header.Alpha[:], buf)
	offset += copy(p.Header.Beta[:], buf[offset:])
	offset += copy(p.Header.Gamma[:], buf[offset:])
	copy(p.Payload[:], buf[offset:])
	return &p
}

// newNodeInfo generates a fresh x25519 key pair for a node bound on conn.
func newNodeInfo(conn *net.UDPConn) (models.NodeInfo, *x25519.PrivateKey, error) {
	publicKey, privateKey, err := x25519.GenerateKey(rand.Reader)
	if err != nil {
		return models.NodeInfo{}, nil, err
	}
	return models.NodeInfo{
		ID:        models.NodeID(*publicKey),
		Address:   conn.LocalAddr().String(),
		PublicKey: publicKey,
	}, privateKey, nil
}

func closeAll(conns []*net.UDPConn) error {
	for _, conn := range conns {
		err := conn.Close()
		if err != nil {
			return fmt.Errorf("We can't close all", err)
		}
	}

	return nil
}
