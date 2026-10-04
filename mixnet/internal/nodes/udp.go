package nodes

import (
	"context"
	"crypto/rand"
	"errors"
	"log"
	"net"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"

	"mixnet/internal/models"
	"mixnet/internal/packets"
)

// serve reads fixed-size packets until ctx is cancelled, dropping malformed datagrams and, with a nil handle, every packet.
func serve(ctx context.Context, name string, conn *net.UDPConn, handle func(*models.SphinxPacket)) error {
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
			return nil
		}
		if n != models.PacketSize || handle == nil {
			continue
		}
		handle(packets.Decode(buf[:n]))
	}
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
