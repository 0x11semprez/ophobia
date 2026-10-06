package bridge

import (
	"bytes"
	"context"
	"net"
	"sync/atomic"
	"testing"
	"time"

	"mixnet/internal/models"
	"mixnet/internal/nodes"
	"mixnet/internal/route"
)

// startMixnet runs a small mixnet with fast clients, retrying on a busy port range.
func startMixnet(t *testing.T, ctx context.Context, clients int) []*nodes.RunningClient {
	t.Helper()
	for attempt := 0; attempt < 20; attempt++ {
		base := 20000 + (time.Now().Nanosecond()/1000+attempt*977)%20000
		var directory atomic.Pointer[route.Directory]
		attemptCtx, cancel := context.WithCancel(ctx)
		mix, err := nodes.LaunchMixNodes(attemptCtx, nodes.MixConfig{
			Host: "127.0.0.1", BasePort: base, Layers: 3, PerLayer: 2,
			MeanDelay: 2 * time.Millisecond, Handle: nodes.RouteMixnode(&directory),
		})
		if err != nil {
			cancel()
			continue
		}
		providers, err := nodes.LaunchProviders(attemptCtx, nodes.ProviderConfig{
			Host: "127.0.0.1", BasePort: base + 10, Count: 2,
			MeanDelay: 2 * time.Millisecond, PullSize: 10, Handle: nodes.RouteProvider(&directory),
		})
		if err != nil {
			cancel()
			mix.Wait()
			continue
		}
		dir := route.NewDirectory(mix.Mixnodes, providers.Providers)
		directory.Store(dir)
		t.Cleanup(func() {
			cancel()
			mix.Wait()
			providers.Wait()
		})

		running := make([]*nodes.RunningClient, 0, clients)
		for i := 0; i < clients; i++ {
			c, err := nodes.LaunchClient(attemptCtx, nodes.ClientConfig{
				Host: "127.0.0.1", Provider: providers.Providers[i%len(providers.Providers)], Directory: dir,
				MeanDelay: 2 * time.Millisecond, PayloadRate: 200,
			})
			if err != nil {
				t.Fatal(err)
			}
			t.Cleanup(c.Wait)
			running = append(running, c)
		}
		return running
	}
	t.Fatal("no free port range for the mixnet")
	return nil
}

func launch(t *testing.T, clients int) (*Bridge, []*nodes.RunningClient) {
	t.Helper()
	ctx, cancel := context.WithCancel(context.Background())
	running := startMixnet(t, ctx, clients)
	b, err := Launch(ctx, "127.0.0.1", 0, running, 10*time.Millisecond)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		cancel()
		b.Wait()
	})
	return b, running
}

func dial(t *testing.T, addr string) (net.Conn, models.NodeID) {
	t.Helper()
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { conn.Close() })
	conn.SetDeadline(time.Now().Add(15 * time.Second))
	kind, payload, err := readMessage(conn)
	if err != nil || kind != msgHello || len(payload) != models.NodeIDSize {
		t.Fatalf("bad hello: kind %d len %d err %v", kind, len(payload), err)
	}
	return conn, models.NodeID(payload)
}

func sendTo(t *testing.T, conn net.Conn, to models.NodeID, data []byte) {
	t.Helper()
	if err := (&writer{conn: conn}).write(msgSend, append(to[:], data...)); err != nil {
		t.Fatal(err)
	}
}

// nextFrame returns the next frame message, skipping other kinds.
func nextFrame(t *testing.T, conn net.Conn) []byte {
	t.Helper()
	for {
		kind, payload, err := readMessage(conn)
		if err != nil {
			t.Fatalf("waiting for a frame: %v", err)
		}
		if kind == msgFrame {
			return payload
		}
	}
}

func TestFrameCrossesTheMixnet(t *testing.T) {
	b, _ := launch(t, 2)
	addrs := b.Addrs()
	alice, _ := dial(t, addrs[0])
	bob, bobID := dial(t, addrs[1])

	want := bytes.Repeat([]byte("ophobia"), 100)
	sendTo(t, alice, bobID, want)

	if got := nextFrame(t, bob); !bytes.Equal(got, want) {
		t.Fatalf("frame changed in transit: got %d bytes, want %d", len(got), len(want))
	}
}

func TestPeersListsTheOtherClients(t *testing.T) {
	b, running := launch(t, 3)
	conn, _ := dial(t, b.Addrs()[0])
	if err := (&writer{conn: conn}).write(msgPeers, nil); err != nil {
		t.Fatal(err)
	}
	var payload []byte
	for {
		kind, p, err := readMessage(conn)
		if err != nil {
			t.Fatal(err)
		}
		if kind == msgPeers {
			payload = p
			break
		}
	}
	want := append(running[1].Client.ID[:], running[2].Client.ID[:]...)
	if !bytes.Equal(payload, want) {
		t.Fatalf("peers = %x, want %x", payload, want)
	}
}

func TestBadSendsAreDropped(t *testing.T) {
	b, _ := launch(t, 2)
	alice, _ := dial(t, b.Addrs()[0])
	bob, bobID := dial(t, b.Addrs()[1])

	sendTo(t, alice, models.NodeID{9}, []byte("nobody"))
	if err := (&writer{conn: alice}).write(msgSend, []byte{1, 2, 3}); err != nil {
		t.Fatal(err)
	}
	sendTo(t, alice, bobID, bytes.Repeat([]byte("x"), models.MaxMessageSize+1))
	sendTo(t, alice, bobID, []byte("still works"))

	if got := nextFrame(t, bob); string(got) != "still works" {
		t.Fatalf("got %q", got)
	}
}

func TestSecondConnectionIsRefusedWhileBusy(t *testing.T) {
	b, _ := launch(t, 1)
	first, _ := dial(t, b.Addrs()[0])
	second, err := net.Dial("tcp", b.Addrs()[0])
	if err != nil {
		t.Fatal(err)
	}
	defer second.Close()
	second.SetDeadline(time.Now().Add(5 * time.Second))
	if _, _, err := readMessage(second); err == nil {
		t.Fatal("second connection should have been closed")
	}
	first.Close()
}
