// Package route builds and peels per-hop routing commands; plaintext stand-in until Sphinx encryption lands.
package route

import (
	"encoding/binary"
	"math/rand"
	"mixnet/internal/models"
	"slices"
	"time"
)

// DropID as the NextHop of a FlagDeliver command marks a drop cover packet, discarded by the egress provider.
var DropID models.NodeID

// Directory is the public view of the network every node and client routes with.
type Directory struct {
	Layers    [][]models.NodeInfo
	Providers []models.NodeInfo
	byID      map[models.NodeID]models.NodeInfo
}

// NewDirectory indexes mixnodes by layer and providers by ID.
func NewDirectory(mixnodes []*models.Mixnode, providers []*models.Provider) *Directory {
	d := &Directory{byID: make(map[models.NodeID]models.NodeInfo)}
	for _, m := range mixnodes {
		for len(d.Layers) <= m.Layer {
			d.Layers = append(d.Layers, nil)
		}
		d.Layers[m.Layer] = append(d.Layers[m.Layer], m.NodeInfo)
		d.byID[m.ID] = m.NodeInfo
	}
	for _, p := range providers {
		d.Providers = append(d.Providers, p.NodeInfo)
		d.byID[p.ID] = p.NodeInfo
	}
	return d
}

// Lookup returns the directory entry of a mixnode or provider.
func (d *Directory) Lookup(id models.NodeID) (models.NodeInfo, bool) {
	info, ok := d.byID[id]
	return info, ok
}

// Path picks one random mixnode per layer, in layer order.
func (d *Directory) Path() []models.NodeInfo {
	path := make([]models.NodeInfo, len(d.Layers))
	for i, layer := range d.Layers {
		path[i] = layer[rand.Intn(len(layer))]
	}
	return path
}

// Commands returns the commands peeled by ingress, each mix layer and egress, with Exp(mean) delays, delivering to recipient.
func Commands(path []models.NodeInfo, egress models.NodeInfo, recipient models.NodeID, mean time.Duration) []models.HopInfo {
	hops := slices.Concat(path, []models.NodeInfo{egress})
	commands := make([]models.HopInfo, 0, len(hops)+1)
	for _, hop := range hops {
		delay := time.Duration(rand.ExpFloat64() * float64(mean))
		commands = append(commands, models.HopInfo{Flag: models.FlagRelay, NextHop: hop.ID, Delay: delay})
	}
	return append(commands, models.HopInfo{Flag: models.FlagDeliver, NextHop: recipient})
}

// Build writes commands into Beta and copies payload, panicking past models.MaxHops.
func Build(commands []models.HopInfo, payload [models.PayloadSize]byte) *models.SphinxPacket {
	if len(commands) > models.MaxHops {
		panic("route: too many hops")
	}
	p := &models.SphinxPacket{Payload: payload}
	for i, c := range commands {
		encode(p.Header.Beta[i*models.HopInfoSize:], c)
	}
	return p
}

// Peel reads the first command from Beta and shifts the rest forward, zero-padding the tail.
func Peel(p *models.SphinxPacket) models.HopInfo {
	c := decode(p.Header.Beta[:models.HopInfoSize])
	copy(p.Header.Beta[:], p.Header.Beta[models.HopInfoSize:])
	clear(p.Header.Beta[models.RoutingInfoSize-models.HopInfoSize:])
	return c
}

func encode(buf []byte, c models.HopInfo) {
	buf[0] = byte(c.Flag)
	offset := models.FlagSize + copy(buf[models.FlagSize:], c.NextHop[:])
	binary.BigEndian.PutUint64(buf[offset:], uint64(c.Delay))
	copy(buf[offset+models.DelaySize:], c.NextMac[:])
}

func decode(buf []byte) models.HopInfo {
	var c models.HopInfo
	c.Flag = models.RoutingFlag(buf[0])
	offset := models.FlagSize + copy(c.NextHop[:], buf[models.FlagSize:])
	c.Delay = time.Duration(binary.BigEndian.Uint64(buf[offset:]))
	copy(c.NextMac[:], buf[offset+models.DelaySize:])
	return c
}
