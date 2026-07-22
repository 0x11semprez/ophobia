// Package network serve to lanch all nodes gave by the user
package network

import "github.com/oasisprotocol/curve25519-voi/primitives/x25519"

type Node struct {
	ID         int
	Port       string
	PrivateKey *x25519.PrivateKey
	PublicKey  *x25519.PublicKey
}
