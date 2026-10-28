package plateauapi

import "strings"

// Choices and variants of PLATEAU dataset items
//
// The items of a PLATEAU dataset differ along two kinds of axes.
//
// A choice is an item that a user picks. Format (3D Tiles / MVT), LOD, texture,
// interior and flooding scale are choices: VIEW lists every item of PlateauDataset.items
// in the dataset's menu by its name (e.g. "LOD2（テクスチャなし）"), and the user selects
// one of them. So items that differ only in these are all returned.
//
// A variant is another version of the same choice, which a user does not need to pick in
// order to see the data. The CRS of the heights (Crs) and the attribute set (AttributeSet)
// are variants. If they were listed as items, VIEW's menu would show the same LOD twice or
// four times. So PlateauDataset.items returns one variant of each choice by default, and
// the others only when they are asked for. PlateauDataset.allItems returns every item.
//
// A new axis is a variant only if it should be hidden from the default listing in the same
// way. If users should be able to pick it in VIEW, it is a choice and is listed as items.

// DefaultCrs is the CRS of the items returned when the caller does not ask for one.
// Switching the published 3D Tiles to JGD2024 means changing this value.
var DefaultCrs = CrsJgd2011

// FullAttributeSetIDSuffix is appended to the ID of an item that has every attribute,
// so that it can be paired with the default item of the same choice.
const FullAttributeSetIDSuffix = "_full"

// FilterPlateauDatasetItems returns the items of the given CRS and attribute set, which
// are the variants described above.
//
// A nil crs means DefaultCrs. Items without a CRS (MVT etc.) are always kept. An item of
// another CRS is never used in place of a missing one, since that would mix geoid models.
//
// A nil attributeSet prefers the default item of each choice and falls back to the
// full one when the choice has no default item.
func FilterPlateauDatasetItems(items []*PlateauDatasetItem, crs *Crs, attributeSet *AttributeSet) []*PlateauDatasetItem {
	targetCrs := DefaultCrs
	if crs != nil {
		targetCrs = *crs
	}

	res := make([]*PlateauDatasetItem, 0, len(items))
	defaults := map[string]struct{}{}
	for _, item := range items {
		if item == nil || item.Crs != nil && *item.Crs != targetCrs {
			continue
		}
		set := item.attributeSetOrDefault()
		if attributeSet != nil && set != *attributeSet {
			continue
		}
		if set == AttributeSetDefault {
			defaults[item.ID.String()] = struct{}{}
		}
		res = append(res, item)
	}

	if attributeSet != nil {
		return res
	}

	filtered := res[:0]
	for _, item := range res {
		if item.attributeSetOrDefault() == AttributeSetFull {
			if _, ok := defaults[strings.TrimSuffix(item.ID.String(), FullAttributeSetIDSuffix)]; ok {
				continue
			}
		}
		filtered = append(filtered, item)
	}
	return filtered
}

func (i *PlateauDatasetItem) attributeSetOrDefault() AttributeSet {
	if i.AttributeSet == nil {
		return AttributeSetDefault
	}
	return *i.AttributeSet
}

// DefaultDatasetItems returns the items of the dataset that are shown when no variant is
// asked for. Only PLATEAU datasets have variants.
func DefaultDatasetItems(d Dataset) []DatasetItem {
	p, ok := d.(*PlateauDataset)
	if !ok || p == nil {
		return d.GetItems()
	}
	items := FilterPlateauDatasetItems(p.Items, nil, nil)
	res := make([]DatasetItem, 0, len(items))
	for _, item := range items {
		res = append(res, item)
	}
	return res
}
