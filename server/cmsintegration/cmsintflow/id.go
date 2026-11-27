package cmsintflow

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/cmsintegrationcommon"
)

var ErrInvalidID = errors.New("invalid fme id")

const idPrefix = "flow"

type ID struct {
	ItemID      string
	ProjectID   string
	FeatureType string
	Type        cmsintegrationcommon.ReqType
	// CRS is the CRS the converted assets must have. When it is set and they have another one,
	// the result is not merged into the item. Empty means any.
	CRS cmsintegrationcommon.CRS
}

func parseID(id, secret string) (ID, error) {
	payload, err := unsignID(id, secret)
	if err != nil {
		return ID{}, err
	}

	// the CRS is the optional 6th part, so that IDs signed before it was added still parse
	s := strings.SplitN(payload, ";", 6)
	if len(s) < 5 || s[0] != idPrefix {
		return ID{}, ErrInvalidID
	}

	res := ID{
		ItemID:      s[1],
		ProjectID:   s[2],
		FeatureType: s[3],
		Type:        cmsintegrationcommon.ReqType(s[4]),
	}
	if len(s) == 6 {
		res.CRS = cmsintegrationcommon.CRS(s[5])
	}
	return res, nil
}

func (i ID) Sign(secret string) string {
	payload := fmt.Sprintf("%s;%s;%s;%s;%s", idPrefix, i.ItemID, i.ProjectID, i.FeatureType, i.Type)
	if i.CRS != "" {
		payload += ";" + string(i.CRS)
	}
	return signID(payload, secret)
}

func signID(payload, secret string) string {
	mac := hmac.New(sha256.New, []byte(secret))
	_, _ = mac.Write([]byte(payload))
	sig := hex.EncodeToString(mac.Sum(nil))
	return fmt.Sprintf("%s:%s", sig, payload)
}

func unsignID(id, secret string) (string, error) {
	_, payload, found := strings.Cut(id, ":")
	if !found {
		return "", ErrInvalidID
	}

	if id != signID(payload, secret) {
		return "", ErrInvalidID
	}

	return payload, nil
}
