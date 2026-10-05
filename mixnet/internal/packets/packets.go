// Package packets converts Sphinx packets to and from their wire form and sends them over UDP.
package packets

import (
	"crypto/rand"
	"mixnet/internal/models"
	"net"
)

// Encode flattens a Sphinx packet into a models.PacketSize buffer.
func Encode(p *models.SphinxPacket) []byte {
	buf := make([]byte, models.PacketSize)
	offset := copy(buf, p.Header.Alpha[:])
	offset += copy(buf[offset:], p.Header.Beta[:])
	offset += copy(buf[offset:], p.Header.Gamma[:])
	copy(buf[offset:], p.Payload[:])
	return buf
}

// Decode splits a models.PacketSize buffer into the Sphinx fields.
func Decode(buf []byte) *models.SphinxPacket {
	var p models.SphinxPacket
	offset := copy(p.Header.Alpha[:], buf)
	offset += copy(p.Header.Beta[:], buf[offset:])
	offset += copy(p.Header.Gamma[:], buf[offset:])
	copy(p.Payload[:], buf[offset:])
	return &p
}

// Random returns a packet of random bytes, which on the wire looks like any encrypted packet.
func Random() *models.SphinxPacket {
	buf := make([]byte, models.PacketSize)
	rand.Read(buf)
	return Decode(buf)
}

// Send dials address and writes the first size bytes of p onto the network.
func Send(address string, p *models.SphinxPacket, size int) error {
	udpAddress, err := net.ResolveUDPAddr("udp", address)
	if err != nil {
		return err
	}

	conn, err := net.DialUDP("udp", nil, udpAddress)
	if err != nil {
		return err
	}

	defer conn.Close()

	buf := Encode(p)
	if size > len(buf) {
		size = len(buf)
	}
	_, err = conn.Write(buf[:size])
	if err != nil {
		return err
	}
	return nil
}
