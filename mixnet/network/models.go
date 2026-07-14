package main

type Node struct {
	ID         int
	Address    string
	PrivateKey [32]byte
	PublicKey  [32]byte
}
