package main

import (
	"log"
	"net/http"
)

func (n Node) Start() {
	log.Fatal(http.ListenAndServe(n.Address, nil))
}
