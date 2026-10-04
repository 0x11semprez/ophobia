package nodes

import (
	"errors"
	"fmt"
	"net"
	"strconv"

	"mixnet/internal/errs"
)

// listenAll binds count consecutive UDP sockets from basePort, closing any it opened if one fails.
func listenAll(host string, basePort, count int) ([]*net.UDPConn, error) {
	conns := make([]*net.UDPConn, 0, count)
	for i := range count {
		address := net.JoinHostPort(host, strconv.Itoa(basePort+i))
		conn, err := listen(address)
		if err != nil {
			closeAll(conns)
			return nil, fmt.Errorf("%w: %s: %w", errs.ErrListen, address, err)
		}
		conns = append(conns, conn)
	}
	return conns, nil
}

func listen(address string) (*net.UDPConn, error) {
	udpAddress, err := net.ResolveUDPAddr("udp", address)
	if err != nil {
		return nil, err
	}
	return net.ListenUDP("udp", udpAddress)
}

// closeAll closes every socket, even after a failure, and joins the errors.
func closeAll(conns []*net.UDPConn) error {
	var err error
	for _, conn := range conns {
		err = errors.Join(err, conn.Close())
	}
	return err
}
