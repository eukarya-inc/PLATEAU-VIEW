package cmsintegrationcommon

import (
	"context"
	"errors"
	"fmt"
	"net/url"
	"path"
	"slices"
	"strings"

	cms "github.com/reearth/reearth-cms-api/go"
)

const (
	// JGD2024AssetSuffix is at the end of the name of an asset converted with JGD2024 heights.
	JGD2024AssetSuffix = "_jgd2024"
	// FullAttributesAssetSuffix is at the end of the name of an asset that has every attribute.
	FullAttributesAssetSuffix = "_full"
)

var assetVariantSuffixes = []string{JGD2024AssetSuffix, FullAttributesAssetSuffix}

// TrimAssetVariants removes the variant suffixes from the end of an asset name, in any order.
// name must not have a file extension.
func TrimAssetVariants(name string) (_ string, jgd2024, full bool) {
	for {
		trimmed := false
		for _, s := range assetVariantSuffixes {
			n, ok := strings.CutSuffix(name, s)
			if !ok {
				continue
			}
			name = n
			trimmed = true
			switch s {
			case JGD2024AssetSuffix:
				jgd2024 = true
			case FullAttributesAssetSuffix:
				full = true
			}
		}
		if !trimmed {
			return name, jgd2024, full
		}
	}
}

// IsJGD2024Asset reports whether the asset file name or URL is of the JGD2024 variant.
func IsJGD2024Asset(nameOrURL string) bool {
	name := nameOrURL
	if u, err := url.Parse(nameOrURL); err == nil && u.Path != "" {
		name = u.Path
	}
	name = path.Base(name)
	name = strings.TrimSuffix(name, path.Ext(name))
	_, jgd2024, _ := TrimAssetVariants(name)
	return jgd2024
}

// MergeConvertedAssets returns the assets of a field after a conversion. The converted
// assets replace the existing assets of the same CRS, and the existing assets of the other
// CRS are kept, so converting into JGD2024 does not remove the JGD2011 assets and vice versa.
//
// names maps every asset ID in existing and converted to its file name or URL. An existing
// asset that is not in names no longer exists in CMS and is removed. The converted assets
// must all be of the same CRS.
func MergeConvertedAssets(existing, converted []string, names map[string]string) ([]string, error) {
	jgd2024, err := convertedCRS(converted, names)
	if err != nil {
		return nil, err
	}

	res := make([]string, 0, len(existing)+len(converted))
	for _, id := range existing {
		if name, ok := names[id]; ok && IsJGD2024Asset(name) != jgd2024 {
			res = append(res, id)
		}
	}
	res = append(res, converted...)
	slices.Sort(res)
	return res, nil
}

// MergeConvertedItems is MergeConvertedAssets for a feature type that keeps its assets in
// groups. converted maps a group key to the converted assets, and newID makes the ID of a
// group that does not exist yet. A group left with no assets is removed.
func MergeConvertedItems(existing []FeatureItemDatum, converted map[string][]string, keys []string, names map[string]string, newID func() string) ([]FeatureItemDatum, error) {
	var all []string
	for _, k := range keys {
		all = append(all, converted[k]...)
	}
	jgd2024, err := convertedCRS(all, names)
	if err != nil {
		return nil, err
	}

	// keys of the result: the converted groups in the order of keys, then the existing
	// groups that the conversion did not output
	byKey := map[string]FeatureItemDatum{}
	for _, d := range existing {
		if _, ok := byKey[d.Key]; !ok {
			byKey[d.Key] = d
		}
	}
	order := make([]string, 0, len(existing)+len(keys))
	seen := map[string]struct{}{}
	for _, k := range keys {
		if _, ok := seen[k]; ok {
			continue
		}
		seen[k] = struct{}{}
		order = append(order, k)
		if _, ok := byKey[k]; !ok {
			byKey[k] = FeatureItemDatum{ID: newID(), Key: k}
		}
	}
	for _, d := range existing {
		if _, ok := seen[d.Key]; !ok {
			seen[d.Key] = struct{}{}
			order = append(order, d.Key)
		}
	}

	res := make([]FeatureItemDatum, 0, len(order))
	for _, k := range order {
		d := byKey[k]
		var data []string
		for _, id := range d.Data {
			if name, ok := names[id]; ok && IsJGD2024Asset(name) != jgd2024 {
				data = append(data, id)
			}
		}
		data = append(data, converted[k]...)
		if len(data) == 0 {
			continue
		}
		slices.Sort(data)
		res = append(res, FeatureItemDatum{ID: d.ID, Key: d.Key, Data: data})
	}
	return res, nil
}

func convertedCRS(converted []string, names map[string]string) (jgd2024 bool, _ error) {
	for i, id := range converted {
		name, ok := names[id]
		if !ok {
			return false, fmt.Errorf("unknown asset name: %s", id)
		}
		j := IsJGD2024Asset(name)
		if i == 0 {
			jgd2024 = j
		} else if j != jgd2024 {
			return false, fmt.Errorf("converted assets mix JGD2011 and JGD2024: %s", name)
		}
	}
	return jgd2024, nil
}

// AssetGetter gets an asset from CMS.
type AssetGetter interface {
	Asset(ctx context.Context, id string) (*cms.Asset, error)
}

// AssetNames returns the file names (or URLs when the name is empty) of the assets.
// Assets that no longer exist in CMS are left out.
func AssetNames(ctx context.Context, c AssetGetter, ids []string) (map[string]string, error) {
	res := make(map[string]string, len(ids))
	for _, id := range ids {
		if _, ok := res[id]; ok || id == "" {
			continue
		}
		a, err := c.Asset(ctx, id)
		if errors.Is(err, cms.ErrNotFound) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("failed to get asset %s: %w", id, err)
		}
		name := a.Name
		if name == "" {
			name = a.URL
		}
		res[id] = name
	}
	return res, nil
}
