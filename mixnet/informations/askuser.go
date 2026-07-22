// Package informations is use to set up all nodes.
package informations

import (
	"fmt"
	"log"
)

func AskNodes(input *int) {
	_, err := fmt.Scan(input)
	if err != nil {
		log.Fatal(err)
	}
}

func AskBasePort(input *int) {
	_, err := fmt.Scan(input)
	if err != nil {
		log.Fatal(err)
	}
}
