package cryptography

import (
	"crypto/rand"

	"github.com/oasisprotocol/curve25519-voi/primitives/x25519"
)

func GenerateKeyPair() (*x25519.PrivateKey, *x25519.PublicKey) {
	privateKey, publicKey, err := x25519.GenerateKey(rand.Reader)
	if err != nil {
		panic(err)
	}

	return privateKey, publicKey
}
