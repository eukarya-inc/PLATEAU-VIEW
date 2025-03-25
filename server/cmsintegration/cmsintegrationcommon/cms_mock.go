package cmsintegrationcommon

import (
	"context"
	"io"

	cms "github.com/reearth/reearth-cms-api/go"
)

type CMSMock struct {
	cms.Interface
	MockGetItem             func(ctx context.Context, id string, asset bool) (*cms.Item, error)
	MockGetItemsPartially   func(ctx context.Context, id string, page, perPage int, asset bool) (*cms.Items, error)
	MockCreateItem          func(ctx context.Context, modelID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error)
	MockUpdateItem          func(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error)
	MockAsset               func(ctx context.Context, id string) (*cms.Asset, error)
	MockUploadAsset         func(ctx context.Context, projectID, url string) (string, error)
	MockUploadAssetDirectly func(ctx context.Context, projectID, name string, r io.Reader, opts ...cms.UploadAssetOption) (string, error)
	MockCommentToItem       func(ctx context.Context, assetID, content string) error
	MockGetModels           func(ctx context.Context, projectID string) (*cms.Models, error)
}

var _ cms.Interface = &CMSMock{}

func (c *CMSMock) Reset() {
	c.MockGetItem = nil
	c.MockGetItemsPartially = nil
	c.MockUpdateItem = nil
	c.MockAsset = nil
	c.MockUploadAsset = nil
	c.MockUploadAssetDirectly = nil
	c.MockCommentToItem = nil
	c.MockGetModels = nil
}

func (c *CMSMock) GetItem(ctx context.Context, id string, asset bool) (*cms.Item, error) {
	return c.MockGetItem(ctx, id, asset)
}

func (c *CMSMock) GetItemsPartially(ctx context.Context, id string, page, perPage int, asset bool) (*cms.Items, error) {
	return c.MockGetItemsPartially(ctx, id, page, perPage, asset)
}

func (c *CMSMock) CreateItem(ctx context.Context, modelID string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
	return c.MockCreateItem(ctx, modelID, fields, metadataFields)
}

func (c *CMSMock) UpdateItem(ctx context.Context, id string, fields []*cms.Field, metadataFields []*cms.Field) (*cms.Item, error) {
	return c.MockUpdateItem(ctx, id, fields, metadataFields)
}

func (c *CMSMock) Asset(ctx context.Context, id string) (*cms.Asset, error) {
	return c.MockAsset(ctx, id)
}

func (c *CMSMock) UploadAsset(ctx context.Context, projectID, url string) (string, error) {
	return c.MockUploadAsset(ctx, projectID, url)
}

func (c *CMSMock) UploadAssetDirectly(ctx context.Context, projectID, name string, r io.Reader, opts ...cms.UploadAssetOption) (string, error) {
	return c.MockUploadAssetDirectly(ctx, projectID, name, r, opts...)
}

func (c *CMSMock) CommentToItem(ctx context.Context, assetID, content string) error {
	return c.MockCommentToItem(ctx, assetID, content)
}

func (c *CMSMock) GetModels(ctx context.Context, projectID string) (*cms.Models, error) {
	return c.MockGetModels(ctx, projectID)
}
