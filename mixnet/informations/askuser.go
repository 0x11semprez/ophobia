// Package informations is use to set up all nodes.
package informations

import (
	"fmt"
	"log"
)

func AskNodes(input int) int {
	nodesNumber, err := fmt.Scan(&input)
	if err != nil {
		log.Fatal(err)
	}
	return nodesNumber
}

func AskBasePort(input int) int {
	basePort, err := fmt.Scan(&input)
	if err != nil {
		log.Fatal(err)
	}
	return basePort
}
