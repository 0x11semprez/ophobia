package main

import (
	"mixnet/internal/cryptography"
	"mixnet/internal/informations"
	"mixnet/internal/network"
	"strconv"
)

func main() {
	var NodesNumber int
	var BasePort int

	informations.AskNodes(&NodesNumber)
	informations.AskBasePort(&BasePort)

	nodes := make([]network.Node, NodesNumber)

	for i := 0; i <= NodesNumber; i++ {
		privateKey, publicKey := cryptography.GenerateKeyPair()

		nodes[i] = network.Node{
			ID:         i,
			Port:       strconv.Itoa(BasePort + i),
			PrivateKey: privateKey,
			PublicKey:  publicKey,
		}

	}
}
