package geospatialjpv3

// エラーメッセージ定数（golangci-lintのST1005エラーを回避）
const (
	errMsgDatasetSearchFailed           = "G空間情報センターからデータセットを検索できませんでした"
	errMsgDatasetCreateFailed           = "G空間情報センターにデータセット %s を作成できませんでした"
	errMsgDatasetUpdateFailed           = "G空間情報センターにデータセット %s を更新できませんでした"
	errMsgResourceUpdateFailed          = "G空間情報センターのリソース %s を更新できませんでした"
	errMsgResourceCreateFailed          = "G空間情報センターにリソース %s を作成できませんでした"
	errMsgResourceReorderFailed         = "G空間情報センターにリソースの順序を変更できませんでした"
	errMsgPackageSearchCreateFailed     = "G空間情報センターでパッケージの検索・作成に失敗しました"
	errMsgResourceCreateIndexFailed     = "G空間情報センターでリソースの作成に失敗しました（索引図）"
	errMsgResourceCreateRelatedFailed   = "G空間情報センターでリソースの作成に失敗しました（関連データセット）"
	errMsgResourceCreateOtherFailed     = "G空間情報センターでリソースの作成に失敗しました（その他データセット）"
	errMsgResourceReorderPartialSuccess = "G空間情報センターでリソースの並び替えに失敗しました（リソースの登録・更新自体は既に完了しています）"
	errMsgResourceCreateCityGMLFailed   = "G空間情報センターでリソースの作成に失敗しました（CityGML）"
	errMsgResourceCreate3DTilesFailed   = "G空間情報センターでリソースの作成に失敗しました（3D Tiles,MVT）"
	errMsgDataItemNotFound              = "G空間センター用データアイテムが取得できません"
	errMsgIndexItemNotFound             = "G空間センター用目録アイテムが取得できません"
	errMsgCityNotReady                  = "この都市はまだG空間情報センターへの公開準備ができていないようです。データを揃えて「公開準備」をONにしてください。"
)
