package cmsintegrationcommon

import (
	"fmt"
	"path"
	"strconv"
	"strings"

	"github.com/oklog/ulid/v2"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
)

const ModelPrefix = "plateau-"
const CityModel = "city"
const RelatedModel = "related"
const GeospatialjpIndex = "geospatialjp-index"
const GeospatialjpData = "geospatialjp-data"

type ManagementStatus string

func (s ManagementStatus) String() string {
	return string(s)
}

const (
	ManagementStatusNotStarted ManagementStatus = "登録未着手"
	ManagementStatusRunning    ManagementStatus = "新規登録中"
	ManagementStatusSkip       ManagementStatus = "対象外"
	ManagementStatusDone       ManagementStatus = "登録済み"
	ManagementStatusReady      ManagementStatus = "確認可能"
)

type ConvertionStatus string

func (s ConvertionStatus) String() string {
	return string(s)
}

const (
	ConvertionStatusNotStarted ConvertionStatus = "未実行"
	ConvertionStatusRunning    ConvertionStatus = "実行中"
	ConvertionStatusError      ConvertionStatus = "エラー"
	ConvertionStatusSuccess    ConvertionStatus = "成功"
)

type CityItem struct {
	ID                string            `json:"id,omitempty" cms:"id"`
	Prefecture        string            `json:"prefecture,omitempty" cms:"prefecture,select"`
	CityName          string            `json:"city_name,omitempty" cms:"city_name,text"`
	CityNameEn        string            `json:"city_name_en,omitempty" cms:"city_name_en,text"`
	CityCode          string            `json:"city_code,omitempty" cms:"city_code,text"`
	Spec              string            `json:"spec,omitempty" cms:"spec,select"`
	OpenDataUrl       string            `json:"open_data_url,omitempty" cms:"open_data_url,url"`
	PRCS              PRCS              `json:"prcs,omitempty" cms:"prcs,select"`
	CodeLists         *cms.PublicAsset  `json:"codelists,omitempty" cms:"codelists,asset"`
	Schemas           *cms.PublicAsset  `json:"schemas,omitempty" cms:"schemas,asset"`
	Metadata          *cms.PublicAsset  `json:"metadata,omitempty" cms:"metadata,asset"`
	Specification     *cms.PublicAsset  `json:"specification,omitempty" cms:"specification,asset"`
	ObjectLists       *cms.PublicAsset  `json:"objectLists,omitempty" cms:"objectLists,asset"`
	Misc              *cms.PublicAsset  `json:"misc,omitempty" cms:"misc,asset"`
	References        map[string]string `json:"references,omitempty" cms:"-"`
	RelatedDataset    string            `json:"related_dataset,omitempty" cms:"related_dataset,reference"`
	GeospatialjpIndex string            `json:"geospatialjp-index,omitempty" cms:"geospatialjp-index,reference"`
	GeospatialjpData  string            `json:"geospatialjp-data,omitempty" cms:"geospatialjp-data,reference"`
	// meatadata
	PlateauDataStatus string          `json:"plateau_data_status,omitempty" cms:"plateau_data_status,select,metadata"`
	CityPublic        bool            `json:"city_public,omitempty" cms:"city_public,bool,metadata"`
	SDKPublic         bool            `json:"sdk_public,omitempty" cms:"sdk_public,bool,metadata"`
	Public            map[string]bool `json:"public,omitempty" cms:"-"`
}

func CityItemFrom(item *cms.Item, featureTypes []string) (i *CityItem) {
	i = &CityItem{}
	item.Unmarshal(i)

	references := map[string]string{}
	public := map[string]bool{}
	for _, ft := range featureTypes {
		if ref := item.FieldByKey(ft).GetValue().String(); ref != nil {
			references[ft] = *ref
		}

		if pub := item.MetadataFieldByKey(ft + "_public").GetValue().Bool(); pub != nil {
			public[ft] = *pub
		}
	}

	i.References = references
	i.Public = public
	return
}

func (i *CityItem) SpecMajorVersionInt() int {
	s := strings.TrimPrefix(i.Spec, "v")
	s = strings.TrimPrefix(s, "第")
	s = strings.TrimSuffix(s, "版")

	m, _, ok := strings.Cut(s, ".")
	if !ok {
		m = s
	}

	v, err := strconv.Atoi(m)
	if err != nil {
		return 0
	}

	return v
}

func (i *CityItem) CMSItem(featureTypes []string) *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)

	for _, ft := range featureTypes {
		if ref, ok := i.References[ft]; ok {
			item.Fields = append(item.Fields, &cms.Field{
				Key:   ft,
				Type:  "reference",
				Value: ref,
			})
		}

		if pub, ok := i.Public[ft]; ok {
			item.MetadataFields = append(item.MetadataFields, &cms.Field{
				Key:   ft + "_public",
				Type:  "bool",
				Value: pub,
			})
		}
	}

	return item
}

func (i *CityItem) ConvSettings() *ConvSettings {
	if i == nil {
		return nil
	}
	var schemas, codeLists, objectLists string
	if i.Schemas != nil {
		schemas = i.Schemas.URL
	}
	if i.CodeLists != nil {
		codeLists = i.CodeLists.URL
	}
	if i.ObjectLists != nil {
		objectLists = i.ObjectLists.URL
	}
	return &ConvSettings{
		PRCS:        i.PRCS.EPSGCode(),
		Schemas:     schemas,
		CodeLists:   codeLists,
		ObjectLists: objectLists,
	}
}

func (i *CityItem) MetadataZipURLs() []string {
	if i == nil {
		return nil
	}

	files := []string{
		i.CodeLists.URL,
		i.Schemas.URL,
		i.Metadata.URL,
		i.Specification.URL,
		i.Misc.URL,
	}

	return lo.Filter(files, func(s string, _ int) bool {
		return path.Ext(s) == ".zip"
	})
}

type FeatureItem struct {
	ID          string             `json:"id,omitempty" cms:"id"`
	City        string             `json:"city,omitempty" cms:"city,reference"`
	CityGML     string             `json:"citygml,omitempty" cms:"citygml,asset"`
	Data        []string           `json:"data,omitempty" cms:"data,asset"`
	Desc        string             `json:"desc,omitempty" cms:"desc,textarea"`
	Items       []FeatureItemDatum `json:"items,omitempty" cms:"items,group"`
	QCResult    string             `json:"qc_result,omitempty" cms:"qc_result,asset"`
	SearchIndex string             `json:"search_index,omitempty" cms:"search_index,asset"`
	Dic         string             `json:"dic,omitempty" cms:"dic,textarea"`
	MaxLOD      string             `json:"maxlod,omitempty" cms:"maxlod,asset"`
	FeatureType string             `json:"featureType,omitempty" cms:"feature_type,select"`

	// override city item's settings
	PRCS        string `json:"prcs" cms:"prcs,text"`
	Schemas     string `json:"schemas" cms:"schemas,asset"`
	CodeLists   string `json:"codelists" cms:"code_lists,asset"`
	ObjectLists string `json:"objectLists" cms:"object_lists,asset"`

	// metadata
	SkipQCConv       *cms.Tag `json:"skip_qc_conv,omitempty" cms:"skip_qc_conv,tag,metadata"`
	Status           *cms.Tag `json:"status,omitempty" cms:"status,select,metadata"`
	ConvertionStatus *cms.Tag `json:"conv_status,omitempty" cms:"conv_status,tag,metadata"`
	QCStatus         *cms.Tag `json:"qc_status,omitempty" cms:"qc_status,tag,metadata"`

	// compat
	SkipQC      bool `json:"skip_qc,omitempty" cms:"skip_qc,bool,metadata"`
	SkipConvert bool `json:"skip_conv,omitempty" cms:"skip_conv,bool,metadata"`
}

func (f *FeatureItem) FeatureTypeCode() string {
	if f == nil || f.FeatureType == "" {
		return ""
	}
	// remain only alphabet
	return strings.Map(func(r rune) rune {
		if r >= 'a' && r <= 'z' {
			return r
		}
		return -1
	}, f.FeatureType)
}

func (f *FeatureItem) ConvSettings() *ConvSettings {
	if f == nil {
		return nil
	}
	return &ConvSettings{
		FeatureType: f.FeatureTypeCode(),
		PRCS:        f.PRCS,
		Schemas:     f.Schemas,
		CodeLists:   f.CodeLists,
		ObjectLists: f.ObjectLists,
	}
}

type ConvSettings struct {
	FeatureType string `json:"featureType"`
	PRCS        string `json:"prcs"`
	Schemas     string `json:"schemas"`
	CodeLists   string `json:"codelists"`
	ObjectLists string `json:"objectLists"`
}

func (c *ConvSettings) Merge(other *ConvSettings) *ConvSettings {
	if c == nil {
		return other
	}
	if other == nil {
		return c
	}
	if other.FeatureType != "" {
		c.FeatureType = other.FeatureType
	}
	if other.PRCS != "" {
		c.PRCS = other.PRCS
	}
	if other.Schemas != "" {
		c.Schemas = other.Schemas
	}
	if other.CodeLists != "" {
		c.CodeLists = other.CodeLists
	}
	if other.ObjectLists != "" {
		c.ObjectLists = other.ObjectLists
	}
	return c
}

func (c *ConvSettings) Validate(qc bool) error {
	if c == nil {
		return fmt.Errorf("変換設定が見つかりません。")
	}
	if c.FeatureType == "" {
		return fmt.Errorf("地物型が不明です。")
	}
	if c.Schemas == "" {
		return fmt.Errorf("schemasが登録されていません。")
	}
	if c.CodeLists == "" {
		return fmt.Errorf("codelistsが登録されていません。")
	}
	if qc {
		if c.PRCS == "" {
			return fmt.Errorf("PRCSの指定が必要です。")
		}
		if c.ObjectLists == "" {
			return fmt.Errorf("objectListsが登録されていません。")
		}
	}
	return nil
}

type FeatureItemDatum struct {
	ID   string   `json:"id,omitempty" cms:"id"`
	Data []string `json:"data,omitempty" cms:"data,asset"`
	Name string   `json:"name,omitempty" cms:"name,text"`
	Desc string   `json:"desc,omitempty" cms:"desc,textarea"`
	Key  string   `json:"key,omitempty" cms:"key,text"`
}

func FeatureItemFrom(item *cms.Item) (i *FeatureItem) {
	i = &FeatureItem{}
	item.Unmarshal(i)
	return
}

func (i *FeatureItem) CMSItem() *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)
	return item
}

type GenericItem struct {
	ID          string               `json:"id,omitempty" cms:"id"`
	City        string               `json:"city,omitempty" cms:"city,reference"`
	Name        string               `json:"name,omitempty" cms:"name,text"`
	Type        string               `json:"type,omitempty" cms:"type,text"`
	TypeEn      string               `json:"type_en,omitempty" cms:"type_en,text"`
	Datasets    []GenericItemDataset `json:"datasets,omitempty" cms:"datasets,group"`
	OpenDataUrl string               `json:"open-data-url,omitempty" cms:"open_data_url,url"`
	Year        string               `json:"year,omitempty" cms:"year,select"`
	// metadata
	Public bool `json:"public,omitempty" cms:"public,bool,metadata"`
	UseAR  bool `json:"use-ar,omitempty" cms:"use-ar,bool,metadata"`
}

type GenericItemDataset struct {
	ID         string `json:"id,omitempty" cms:"id"`
	Data       string `json:"data,omitempty" cms:"data,asset"`
	Desc       string `json:"desc,omitempty" cms:"desc,textarea"`
	DataURL    string `json:"url,omitempty" cms:"data_url,url"`
	DataFormat string `json:"data-format,omitempty" cms:"data_format,select"`
	LayerName  string `json:"layer-name,omitempty" cms:"layer_name,text"`
}

func GenericItemFrom(item *cms.Item) (i *GenericItem) {
	i = &GenericItem{}
	item.Unmarshal(i)
	return
}

func (i *GenericItem) CMSItem() *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)
	return item
}

type RelatedItem struct {
	ID     string                      `json:"id,omitempty" cms:"id"`
	City   string                      `json:"city,omitempty" cms:"city,reference"`
	Items  map[string]RelatedItemDatum `json:"items,omitempty" cms:"-"`
	Merged string                      `json:"merged,omitempty" cms:"merged,asset"`
	// metadata
	Status        *cms.Tag            `json:"status,omitempty" cms:"status,tag,metadata"`
	ConvertStatus map[string]*cms.Tag `json:"conv_status,omitempty" cms:"-"`
	MergeStatus   *cms.Tag            `json:"merge_status,omitempty" cms:"merge_status,tag,metadata"`
	Merge         bool                `json:"merge,omitempty" cms:"merge,bool,metadata"`
}

type RelatedItemDatum struct {
	ID          string   `json:"id,omitempty" cms:"id"`
	Asset       []string `json:"asset,omitempty" cms:"asset,asset"`
	Converted   []string `json:"converted,omitempty" cms:"converted,asset"`
	Description string   `json:"description,omitempty" cms:"description,textarea"`
}

func RelatedItemFrom(item *cms.Item, relatedDataTypes []string) (i *RelatedItem) {
	i = &RelatedItem{}
	item.Unmarshal(i)

	if i.Items == nil {
		i.Items = map[string]RelatedItemDatum{}
	}
	if i.ConvertStatus == nil {
		i.ConvertStatus = map[string]*cms.Tag{}
	}

	for _, t := range relatedDataTypes {
		g := item.FieldByKey(t).GetValue().String()
		if g == nil {
			continue
		}

		if group := item.Group(*g); group != nil && len(group.Fields) > 0 {
			i.Items[t] = RelatedItemDatum{
				ID:          group.ID,
				Asset:       group.FieldByKey("asset").GetValue().Strings(),
				Converted:   group.FieldByKey("conv").GetValue().Strings(),
				Description: lo.FromPtr(group.FieldByKey("description").GetValue().String()),
			}
		}

		tag := item.MetadataFieldByKey(t + "_status").GetValue().Tag()
		if tag != nil {
			i.ConvertStatus[t] = tag
		}
	}

	return
}

func (i *RelatedItem) CMSItem(relatedDataTypes []string) *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)

	for _, t := range relatedDataTypes {
		if d, ok := i.Items[t]; ok {
			if d.ID == "" {
				d.ID = ulid.Make().String()
			}

			if len(d.Asset) > 0 {
				item.Fields = append(item.Fields, &cms.Field{
					Key:   "asset",
					Type:  "asset",
					Value: d.Asset,
					Group: d.ID,
				})
			}

			if len(d.Converted) > 0 {
				item.Fields = append(item.Fields, &cms.Field{
					Key:   "conv",
					Type:  "asset",
					Value: d.Converted,
					Group: d.ID,
				})
			}

			if d.Description != "" {
				item.Fields = append(item.Fields, &cms.Field{
					Key:   "description",
					Type:  "textarea",
					Value: d.Description,
					Group: d.ID,
				})
			}

			item.Fields = append(item.Fields, &cms.Field{
				Key:   t,
				Type:  "group",
				Value: d.ID,
			})
		}

		if tag, ok := i.ConvertStatus[t]; ok {
			item.MetadataFields = append(item.MetadataFields, &cms.Field{
				Key:   t + "_status",
				Type:  "tag",
				Value: tag.Name,
			})
		}
	}

	return item
}

type GeospatialjpIndexItem struct {
	ID        string `json:"id,omitempty" cms:"id"`
	City      string `json:"city,omitempty" cms:"city,reference"`
	Title     string `json:"title,omitempty" cms:"title,text"`
	Desc      string `json:"desc,omitempty" cms:"desc,markdown"`
	Region    string `json:"region,omitempty" cms:"region,text"`
	Thumbnail string `json:"thumbnail,omitempty" cms:"thumbnail,asset"`
	// metadata
	Status ManagementStatus `json:"status,omitempty" cms:"status,select,metadata"`
}

func GeospatialjpIndexItemFrom(item *cms.Item) (i *GeospatialjpIndexItem) {
	i = &GeospatialjpIndexItem{}
	item.Unmarshal(i)
	return
}

func (i *GeospatialjpIndexItem) CMSItem() *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)
	return item
}

type GeospatialjpDataItem struct {
	ID          string `json:"id,omitempty" cms:"id"`
	City        string `json:"city,omitempty" cms:"city,reference"`
	CityGML     string `json:"citygml,omitempty" cms:"citygml,asset"`
	PlateauData string `json:"converted-data,omitempty" cms:"plateau_data,asset"`
	RelatedData string `json:"related-data,omitempty" cms:"related_data,asset"`
	GenericData string `json:"generic-data,omitempty" cms:"generic_data,asset"`
	// metadata
	CityGMLMergeStatus        ConvertionStatus `json:"citygml_merge_status,omitempty" cms:"citygml_merge_status,select,metadata"`
	ConvertedDataMergeSatatus ConvertionStatus `json:"plateau_merge_status,omitempty" cms:"plateau_merge_status,select,metadata"`
	RelatedDataMergeStatus    ConvertionStatus `json:"related_merge_status,omitempty" cms:"related_merge_status,select,metadata"`
}

func GeospatialjpDataItemFrom(item *cms.Item) (i *GeospatialjpDataItem) {
	i = &GeospatialjpDataItem{}
	item.Unmarshal(i)
	return
}

func (i *GeospatialjpDataItem) CMSItem() *cms.Item {
	item := &cms.Item{}
	cms.Marshal(i, item)
	return item
}

func (item *FeatureItem) ReqType() ReqType {
	return ReqTypeFrom(item.IsQCAndConvSkipped())
}

func (item *FeatureItem) IsQCAndConvSkipped() (skipQC bool, skipConv bool) {
	const (
		skip = "スキップ"
		qc   = "品質検査"
		conv = "変換"
	)

	if TagIsNot(item.QCStatus, ConvertionStatusNotStarted) {
		skipQC = true
	}
	if TagIsNot(item.ConvertionStatus, ConvertionStatusNotStarted) {
		skipConv = true
	}

	if skipQC && skipConv {
		return true, true
	}

	if item.SkipQCConv != nil {
		if n := item.SkipQCConv.Name; strings.Contains(n, skip) {
			qc := strings.Contains(n, qc)
			conv := strings.Contains(n, conv)
			if !qc && !conv {
				skipQC = true
				skipConv = true
			} else {
				skipQC = skipQC || qc
				skipConv = skipConv || conv
			}
		}
	}

	skipQC = skipQC || item.SkipQC
	skipConv = skipConv || item.SkipConvert
	return
}

type ReqType string

const (
	ReqTypeQC     ReqType = "qc"
	ReqTypeConv   ReqType = "conv"
	ReqTypeQCConv ReqType = "qc_conv"
)

func ReqTypeFrom(skipQC, skipConv bool) ReqType {
	if skipQC && skipConv {
		return ""
	} else if skipQC {
		return ReqTypeConv
	} else if skipConv {
		return ReqTypeQC
	}
	return ReqTypeQCConv
}

func (r ReqType) IsQC() bool {
	return r == ReqTypeQC || r == ReqTypeQCConv
}

func (r ReqType) IsConv() bool {
	return r == ReqTypeConv || r == ReqTypeQCConv
}

func (r ReqType) Title() string {
	switch r {
	case ReqTypeConv:
		return "変換"
	case ReqTypeQC:
		return "品質検査"
	}
	return "品質検査・変換"
}

func (t ReqType) CMSStatus(s ConvertionStatus) (qc ConvertionStatus, conv ConvertionStatus) {
	if t == ReqTypeConv {
		conv = s
	} else if t == ReqTypeQC {
		qc = s
	} else {
		qc = s
		conv = s
	}
	return
}

func (ty ReqType) Override(override ReqType) ReqType {
	if override != "" {
		return override
	}
	return ty
}

func (ty ReqType) Intersection(other ReqType) ReqType {
	intersectionQC := ty.IsQC() && other.IsQC()
	intersectionConv := ty.IsConv() && other.IsConv()
	if intersectionQC && intersectionConv {
		return ReqTypeQCConv
	}
	if intersectionQC {
		return ReqTypeQC
	}
	if intersectionConv {
		return ReqTypeConv
	}
	return ""
}

func (ty ReqType) Normalize() ReqType {
	if ty == ReqTypeQCConv {
		return ReqTypeQC
	}
	return ty
}
