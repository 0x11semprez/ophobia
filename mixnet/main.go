package main

import (
	"crypto/rsa"
	"mixnet/informations"
	"mixnet/network"
)

func main() {
	var NodesNumber int
	var BasePort int


	informations.AskNodes(NodesNumber)
	informations.AskBasePort(BasePort)

	nodes := make([]network.Node, NodesNumber)

	for i := 0; i <= NodesNumber; i++ {
		i.append(nodes, Node{
			ID: i,
			Address: BasePort + i
			PrivateKey: 
			PublicKey:
		})
	}
}
