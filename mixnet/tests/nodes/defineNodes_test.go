package nodes

import (
	"mixnet/internal/models"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func defineNode() models.NodeInfo {
	return models.NodeInfo{
		ID:      models.NodeID{1},
		Address: "127.0.0.1:9000",
	}
}

func TestDefineNode(t *testing.T) {
	node := defineNode()

	assert.Equal(t, models.NodeID{1}, node.ID)
	assert.Equal(t, "127.0.0.1:9000", node.Address)
	assert.Nil(t, node.PublicKey)
}

func TestDefineMixnode(t *testing.T) {
	mixnode := models.Mixnode{
		NodeInfo:  defineNode(),
		Layer:     1,
		MeanDelay: 50 * time.Millisecond,
		LoopRate:  2.0,
	}

	assert.Equal(t, defineNode(), mixnode.NodeInfo)
	assert.Equal(t, 1, mixnode.Layer)
	assert.Equal(t, 50*time.Millisecond, mixnode.MeanDelay)
	assert.Equal(t, 2.0, mixnode.LoopRate)
}

func TestDefineProvider(t *testing.T) {
	provider := &models.Provider{
		NodeInfo: defineNode(),
		PullSize: 4,
		Inboxes:  map[models.NodeID]*models.Inbox{},
	}
	client := models.NodeID{2}

	assert.Nil(t, provider.Inbox(client))
	provider.Register(client)
	assert.NotNil(t, provider.Inbox(client))
	assert.Equal(t, 4, provider.PullSize)
}
