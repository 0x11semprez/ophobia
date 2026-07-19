// Package network serve to lanch all nodes gave by the user
package network

type Node struct {
	ID         int
	Address    string
	PrivateKey [32]byte
	PublicKey  [32]byte
}
