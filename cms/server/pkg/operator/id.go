package operator

import (
	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/pkg/id"
	"github.com/reearth/reearthx/account/accountdomain"
)

type ID = id.EventID
type UserID = accountdomain.UserID
type IntegrationID = id.IntegrationID

var ErrInvalidID = id.ErrInvalidID
var NewIntegrationID = id.NewIntegrationID
