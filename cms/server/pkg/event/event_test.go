package event

import (
	"testing"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/asset"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/id"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/operator"
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/project"
	"github.com/reearth/reearthx/account/accountdomain/user"
	"github.com/stretchr/testify/assert"
)

func TestEvent(t *testing.T) {
	u := user.New().NewID().Email("hoge@example.com").Name("John").MustBuild()
	a := asset.New().NewID().Thread(id.NewThreadID().Ref()).NewUUID().
		Project(project.NewID()).Size(100).CreatedByUser(u.ID()).MustBuild()
	now := time.Now()
	eID := NewID()
	ev := New[*asset.Asset]().ID(eID).Timestamp(now).Type(AssetCreate).
		Operator(operator.OperatorFromUser(u.ID())).Object(a).MustBuild()

	assert.Equal(t, eID, ev.ID())
	assert.Equal(t, Type(AssetCreate), ev.Type())
	assert.Equal(t, operator.OperatorFromUser(u.ID()), ev.Operator())
	assert.Equal(t, a, ev.Object())
	assert.Equal(t, now, ev.Timestamp())
	assert.Equal(t, ev, ev.Clone())
	assert.NotSame(t, ev, ev.Clone())
}
