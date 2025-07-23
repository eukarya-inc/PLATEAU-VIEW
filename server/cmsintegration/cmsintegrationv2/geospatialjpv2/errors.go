package geospatialjpv2

// エラーメッセージ定数（golangci-lintのST1005エラーを回避）
const (
	errMsgMetadataSheetNotFound         = "G空間情報センター用メタデータシートが見つかりません。"
	errMsgCatalogResourceRegisterFailed = "G空間情報センターへの目録リソースの登録に失敗しました"
	errMsgCityGMLResourceRegisterFailed = "G空間情報センターへのCityGMLリソースの登録に失敗しました"
	errMsgAllDataResourceRegisterFailed = "G空間情報センターへの全データリソースの登録に失敗しました"
	errMsgDatasetSearchFailed           = "G空間情報センターからデータセットを検索できませんでした"
	errMsgDatasetCreateFailed           = "G空間情報センターにデータセット %s を作成できませんでした"
)
