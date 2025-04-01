package cmsintegrationv3

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"testing"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestSendRequestToFME(t *testing.T) {
	ctx := context.Background()
	c := &cmsintegrationcommon.CMSMock{}
	pc := &plateauCMSMock{
		plateauSpecs: func(ctx context.Context) ([]plateaucms.PlateauSpec, error) {
			return []plateaucms.PlateauSpec{
				{
					ID:              "1",
					MajorVersion:    3,
					Year:            2023,
					MaxMinorVersion: 5,
					FMEURL:          "https://example.com/v3",
				},
			}, nil
		},
	}
	f := &fmeMock{}
	s := &Services{
		CMS:     c,
		PCMS:    pc,
		mockFME: f,
	}
	conf := &Config{
		Secret: "secret",
	}
	baseItem := &cms.Item{
		ID:             "itemID",
		MetadataItemID: lo.ToPtr("metadataItemID"),
		Fields: []*cms.Field{
			{
				Key:   "citygml",
				Value: "citygmlID",
			},
			{
				Key:   "city",
				Value: "cityID",
			},
		},
	}
	cityItem := &cms.Item{
		ID: "cityID",
		Fields: []*cms.Field{
			{
				Key: "codelists",
				Value: &cms.Asset{
					ID:  "codelistID",
					URL: "codelists",
				},
			},
			{
				Key:   "citygml",
				Value: "citygmlID",
			},
			{
				Key:   "spec",
				Value: "第3.5版",
			},
		},
	}
	cityItemWithoutCodelists := &cms.Item{
		ID: "cityID",
		Fields: []*cms.Field{
			{
				Key: "codelists",
			},
			{
				Key:   "citygml",
				Value: "citygmlID",
			},
			{
				Key:   "spec",
				Value: "第3.5版",
			},
		},
	}
	w := &cmswebhook.Payload{
		ItemData: &cmswebhook.ItemData{
			Model: &cms.Model{
				Key: "plateau-bldg",
			},
			Item: baseItem,
			Schema: &cms.Schema{
				ProjectID: "projectID",
			},
		},
	}

	t.Run("no metadataItemID and originalItemID", func(t *testing.T) {
		item := *baseItem
		item.MetadataItemID = nil
		item.OriginalItemID = nil
		w.ItemData.Item = &item

		c.Reset()

		err := sendRequestToFME(ctx, s, conf, w)
		assert.ErrorContains(t, err, "invalid webhook payload")
	})

	t.Run("already converted", func(t *testing.T) {
		log := getLogs(t)

		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
					Fields: []*cms.Field{
						{
							Key:   "qc_status",
							Value: &cms.Tag{Name: "成功"},
						},
						{
							Key:   "conv_status",
							Value: &cms.Tag{Name: "成功"},
						},
					},
				}, nil
			}
			return nil, fmt.Errorf("failed to get item")
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)
		assert.Contains(t, log(), "skip qc and convert")
	})

	t.Run("skip qc and convert", func(t *testing.T) {
		log := getLogs(t)

		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}

			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
					Fields: []*cms.Field{
						{
							Key:   "skip_conv",
							Value: true,
						},
						{
							Key:   "skip_qc",
							Value: true,
						},
					},
				}, nil
			}

			return nil, fmt.Errorf("failed to get item")
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)

		assert.Contains(t, log(), "skip qc and convert")
	})

	t.Run("skip qc and convert with tags", func(t *testing.T) {
		log := getLogs(t)

		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}

			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
					Fields: []*cms.Field{
						{
							Key: "skip_qc_conv",
							Value: &cms.Tag{
								Name: "スキップ",
							},
						},
					},
				}, nil
			}

			return nil, fmt.Errorf("failed to get item")
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)

		assert.Contains(t, log(), "skip qc and convert")
	})

	t.Run("failed to get citygml asset", func(t *testing.T) {
		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
				}, nil
			}
			return cityItem, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			return nil, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			return nil, fmt.Errorf("failed to get citygml asset")
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "CityGMLが見つかりません。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.ErrorContains(t, err, "failed to get citygml asset")
	})

	t.Run("failed to get city item", func(t *testing.T) {
		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
				}, nil
			}
			return nil, fmt.Errorf("failed to get city item")
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			return nil, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			return &cms.Asset{
				ID: "citygmlID",
			}, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "都市アイテムが見つかりません。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.ErrorContains(t, err, "failed to get city item")
	})

	t.Run("failed to get codelist asset", func(t *testing.T) {
		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
				}, nil
			}
			return cityItemWithoutCodelists, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			return nil, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			if id == "citygmlID" {
				return &cms.Asset{
					ID: "citygmlID",
				}, nil
			}
			return nil, fmt.Errorf("failed to get asset")
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "コードリストが")
			assert.Contains(t, content, "ません。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.ErrorContains(t, err, "city item has no codelist")
	})

	t.Run("success", func(t *testing.T) {
		f.called = nil

		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
				}, nil
			}
			return cityItem, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			if id == "citygmlID" {
				return &cms.Asset{
					ID:  "citygmlID",
					URL: "target",
				}, nil
			}
			return nil, fmt.Errorf("failed to get asset")
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			return "asset", nil
		}
		c.MockUploadAssetDirectly = func(ctx context.Context, projectID, name string, r io.Reader, opts ...cms.UploadAssetOption) (string, error) {
			return "assetd", nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, "conv_status", metadataFields[0].Key)
			assert.Equal(t, string(cmsintegrationcommon.ConvertionStatusRunning), metadataFields[0].Value)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査・変換を開始しました。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)
		assert.Equal(t, []fmeRequest{
			{
				Type: "qc_conv",
				ID: fmeID{
					ItemID:      "itemID",
					ProjectID:   "projectID",
					FeatureType: "bldg",
					Type:        "qc_conv",
				}.String("secret"),
				Target:    "target",
				Codelists: "codelists",
				ResultURL: "/notify_fme/v3",
			},
		}, f.called)
	})

	t.Run("success with only qc", func(t *testing.T) {
		f.called = nil

		item := *baseItem
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
					Fields: []*cms.Field{
						{
							Key: "skip_qc_conv",
							Value: &cms.Tag{
								Name: "変換をスキップ",
							},
						},
					},
				}, nil
			}
			return cityItem, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			if id == "citygmlID" {
				return &cms.Asset{
					ID:  "citygmlID",
					URL: "target",
				}, nil
			}
			return &cms.Asset{
				ID:  "codelistID",
				URL: "codelists",
			}, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			return "asset", nil
		}
		c.MockUploadAssetDirectly = func(ctx context.Context, projectID, name string, r io.Reader, opts ...cms.UploadAssetOption) (string, error) {
			return "assetd", nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, "qc_status", metadataFields[0].Key)
			assert.Equal(t, string(cmsintegrationcommon.ConvertionStatusRunning), metadataFields[0].Value)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査を開始しました。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)
		assert.Equal(t, []fmeRequest{
			{
				Type: "qc",
				ID: fmeID{
					ItemID:      "itemID",
					ProjectID:   "projectID",
					FeatureType: "bldg",
					Type:        "qc",
				}.String("secret"),
				Target:    "target",
				Codelists: "codelists",
				ResultURL: "/notify_fme/v3",
			},
		}, f.called)
	})

	t.Run("success with metadata item", func(t *testing.T) {
		f.called = nil

		item := *baseItem
		item.ID = "metadataItemID"
		item.MetadataItemID = nil
		item.OriginalItemID = lo.ToPtr("itemID")
		item.IsMetadata = true
		item.Fields = []*cms.Field{
			{
				ID:  "cs",
				Key: "conv_status",
			},
		}
		w.ItemData.Changes = []cms.FieldChange{
			{
				ID: "cs",
			},
		}
		w.ItemData.Item = &item

		c.Reset()
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			if id == "itemID" {
				i := *baseItem
				return &i, nil
			}
			if id == "metadataItemID" {
				return &cms.Item{
					ID: "metadataItemID",
				}, nil
			}
			return cityItem, nil
		}
		c.MockAsset = func(ctx context.Context, id string) (*cms.Asset, error) {
			if id == "citygmlID" {
				return &cms.Asset{
					ID:  "citygmlID",
					URL: "target",
				}, nil
			}
			return &cms.Asset{
				ID:  "codelistID",
				URL: "codelists",
			}, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			return "asset", nil
		}
		c.MockUploadAssetDirectly = func(ctx context.Context, projectID, name string, r io.Reader, opts ...cms.UploadAssetOption) (string, error) {
			return "assetd", nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, "conv_status", metadataFields[0].Key)
			assert.Equal(t, string(cmsintegrationcommon.ConvertionStatusRunning), metadataFields[0].Value)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査・変換を開始しました。")
			return nil
		}

		err := sendRequestToFME(ctx, s, conf, w)
		assert.NoError(t, err)
		assert.Equal(t, []fmeRequest{
			{
				Type: "qc_conv",
				ID: fmeID{
					ItemID:      "itemID",
					ProjectID:   "projectID",
					FeatureType: "bldg",
					Type:        "qc_conv",
				}.String("secret"),
				Target:    "target",
				Codelists: "codelists",
				ResultURL: "/notify_fme/v3",
			},
		}, f.called)
	})
}

func mockGenerateID(t *testing.T) func() {
	t.Helper()
	counter := 0
	originalGenID := generateID
	generateID = func() string {
		counter++
		return fmt.Sprintf("id%d", counter)
	}
	t.Cleanup(func() {
		generateID = originalGenID
	})
	return func() {
		counter = 0
	}
}

func TestReceiveResultFromFME(t *testing.T) {
	mockGenerateID(t)
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	httpmock.RegisterResponder("GET", "https://example.com/dic",
		httpmock.NewStringResponder(200, "dic!!"))

	ctx := context.Background()
	c := &cmsintegrationcommon.CMSMock{}
	pc := &plateauCMSMock{}
	s := &Services{
		CMS:  c,
		PCMS: pc,
	}
	conf := &Config{
		Secret: "secret",
	}
	res := &fmeResult{
		Type: "conv",
		ID: fmeID{
			ItemID:      "itemID",
			ProjectID:   "projectID",
			FeatureType: "bldg",
			Type:        "qc_conv",
		}.String("secret"),
		LogURL: "log_ok",
		Results: map[string]any{
			"_dic":       "https://example.com/dic",
			"_maxlod":    "maxlod",
			"_qc_result": "qc_result",
			"bldg":       "bldg",
		},
		Status: "success",
	}

	t.Run("no items", func(t *testing.T) {
		c.Reset()
		uploaded := []string{}
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			return &cms.Item{
				ID: "itemID",
			}, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			assert.Equal(t, projectID, "projectID")
			uploaded = append(uploaded, url)
			return url, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			assert.Equal(t, []*cms.Field{
				{
					Key:   "data",
					Type:  "asset",
					Value: []string{"bldg"},
				},
				{
					Key:   "qc_result",
					Type:  "asset",
					Value: "qc_result",
				},
				{
					Key:   "dic",
					Type:  "textarea",
					Value: "dic!!",
				},
				{
					Key:   "maxlod",
					Type:  "asset",
					Value: "maxlod",
				},
			}, fields)
			assert.Equal(t, []*cms.Field{
				{
					Key:   "conv_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
				{
					Key:   "qc_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
			}, metadataFields)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査・変換が完了しました。")
			return nil
		}

		err := receiveResultFromFME(ctx, s, conf, *res)
		assert.NoError(t, err)
		assert.Equal(t, []string{"bldg", "maxlod", "qc_result"}, uploaded)
	})

	t.Run("with items", func(t *testing.T) {
		r := *res
		r.ID = fmeID{
			ItemID:      "itemID",
			ProjectID:   "projectID",
			FeatureType: "fld",
			Type:        "qc_conv",
		}.String("secret")
		r.Results = map[string]any{
			"fld/aaa": "AAA",
			"fld/bbb": "BBB",
			"fld/ccc": []string{"CCC", "DDD"},
		}
		c.Reset()
		uploaded := []string{}
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			return (&cmsintegrationcommon.FeatureItem{
				ID: "itemID",
				Items: []cmsintegrationcommon.FeatureItemDatum{
					{
						ID:   "ida",
						Key:  "fld/aaa",
						Data: []string{"data"},
						Name: "name1",
						Desc: "desc1",
					},
					{
						ID:   "idb",
						Key:  "fld/ccc",
						Data: []string{"data"},
						Name: "name2",
						Desc: "desc2",
					},
				},
			}).CMSItem(), nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			assert.Equal(t, projectID, "projectID")
			uploaded = append(uploaded, url)
			return url, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			assert.Equal(t, []*cms.Field{
				{
					Key:   "data",
					Type:  "asset",
					Value: []string{"AAA"},
					Group: "ida",
				},
				{
					Key:   "key",
					Type:  "text",
					Value: "fld/aaa",
					Group: "ida",
				},
				{
					Key:   "data",
					Type:  "asset",
					Value: []string{"BBB"},
					Group: "id1",
				},
				{
					Key:   "key",
					Type:  "text",
					Value: "fld/bbb",
					Group: "id1",
				},
				{
					Key:   "data",
					Type:  "asset",
					Value: []string{"CCC", "DDD"},
					Group: "idb",
				},
				{
					Key:   "key",
					Type:  "text",
					Value: "fld/ccc",
					Group: "idb",
				},
				{
					Key:   "items",
					Type:  "group",
					Value: []string{"ida", "id1", "idb"},
				},
			}, fields)
			assert.Equal(t, []*cms.Field{
				{
					Key:   "conv_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
				{
					Key:   "qc_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
			}, metadataFields)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査・変換が完了しました。")
			return nil
		}

		err := receiveResultFromFME(ctx, s, conf, r)
		assert.NoError(t, err)
		assert.Equal(t, []string{"AAA", "BBB", "CCC", "DDD"}, uploaded)
	})

	t.Run("invalid id", func(t *testing.T) {
		r := *res
		r.ID = "invalid"
		err := receiveResultFromFME(ctx, s, conf, r)
		assert.ErrorContains(t, err, "invalid id")
	})

	t.Run("failed convert", func(t *testing.T) {
		r := *res
		r.Status = "error"
		r.LogURL = "log"
		c.Reset()
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, []*cms.Field{
				{
					Key:   "conv_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusError),
				},
				{
					Key:   "qc_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusError),
				},
			}, metadataFields)
			return nil, nil
		}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			assert.Contains(t, content, "品質検査・変換に失敗しました。")
			assert.Contains(t, content, "ログ： log")
			return nil
		}
		err := receiveResultFromFME(ctx, s, conf, r)
		assert.NoError(t, err)
	})

	t.Run("notify", func(t *testing.T) {
		commneted := []string{}
		r := *res
		r.Type = "notify"
		r.LogURL = "log"
		r.Message = "message"
		r.Results = map[string]any{
			"_qc_result": "qc_result",
		}
		c.Reset()
		uploaded := []string{}
		c.MockCommentToItem = func(ctx context.Context, assetID, content string) error {
			commneted = append(commneted, content)
			return nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			uploaded = append(uploaded, url)
			return url, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			return nil, nil
		}
		err := receiveResultFromFME(ctx, s, conf, r)
		assert.NoError(t, err)
		assert.Equal(t, []string{"message ログ： log"}, commneted)
		assert.Equal(t, []string{"qc_result"}, uploaded)
	})

	t.Run("failed to upload asset", func(t *testing.T) {
		c.Reset()
		uploaded := []string{}
		c.MockGetItem = func(ctx context.Context, id string, asset bool) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			return &cms.Item{
				ID: "itemID",
			}, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			assert.Equal(t, projectID, "projectID")
			uploaded = append(uploaded, url)
			return url, nil
		}
		c.MockUpdateItem = func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
			assert.Equal(t, id, "itemID")
			assert.Equal(t, []*cms.Field{
				{
					Key:   "data",
					Type:  "asset",
					Value: []string{"bldg"},
				},
				{
					Key:   "qc_result",
					Type:  "asset",
					Value: "qc_result",
				},
				{
					Key:   "dic",
					Type:  "textarea",
					Value: "dic!!",
				},
				{
					Key:   "maxlod",
					Type:  "asset",
					Value: "maxlod",
				},
			}, fields)
			assert.Equal(t, []*cms.Field{
				{
					Key:   "conv_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
				{
					Key:   "qc_status",
					Type:  "tag",
					Value: string(cmsintegrationcommon.ConvertionStatusSuccess),
				},
			}, metadataFields)
			return nil, nil
		}
		c.MockUploadAsset = func(ctx context.Context, projectID, url string) (string, error) {
			return "", fmt.Errorf("ERR!")
		}
		err := receiveResultFromFME(ctx, s, conf, *res)
		assert.NoError(t, err)
	})
}

func getLogs(t *testing.T) func() string {
	t.Helper()
	buf := &bytes.Buffer{}
	log.SetOutput(io.MultiWriter(buf, os.Stdout))

	t.Cleanup(func() {
		log.SetOutput(os.Stdout)
	})

	return func() string {
		return buf.String()
	}
}

type plateauCMSMock struct {
	PCMS
	plateauSpecs func(ctx context.Context) ([]plateaucms.PlateauSpec, error)
}

func (p *plateauCMSMock) PlateauSpecs(ctx context.Context) ([]plateaucms.PlateauSpec, error) {
	return p.plateauSpecs(ctx)
}

func (p *plateauCMSMock) PlateauFeatureTypes(ctx context.Context) (plateaucms.PlateauFeatureTypeList, error) {
	return []plateaucms.PlateauFeatureType{
		{
			Code: "bldg",
			Name: "建築物モデル",
			QC:   true,
			Conv: true,
		},
		{
			Code: "tran",
			Name: "交通モデル（道路）",
			QC:   true,
			Conv: true,
		},
		{
			Code: "luse",
			Name: "土地利用モデル",
			QC:   true,
			Conv: true,
		},
		{
			Code:      "fld",
			Name:      "洪水浸水想定区域モデル",
			QC:        true,
			Conv:      true,
			UseGroups: true,
		},
	}, nil
}

func (p *plateauCMSMock) Metadata(ctx context.Context, prj string, findDataCatalog, useDefault bool) (plateaucms.Metadata, plateaucms.MetadataList, error) {
	return plateaucms.Metadata{
		Converter: "fme",
	}, nil, nil
}
