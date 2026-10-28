# PLATEAU データ配信のアクセス集計

PLATEAU の 3D 都市モデルなどのデータは、PLATEAU CMS から配信している。PLATEAU VIEW や PLATEAU SDK、G空間情報センターのデータセットページからダウンロードされるファイルも、ここから届く。

このディレクトリには、その配信について、どのデータが・どこから・どれだけ使われ、配信にどれくらいの費用がかかっているかを集計する仕組みの SQL と説明を置く。集計の結果は、データ整備の優先順位の検討や、配信の運用（費用の把握、過剰なアクセスへの対応）に使う。

## 集める情報と扱い

配信サーバーへのリクエストごとに、次の情報が記録される。

- 日時
- 要求されたファイルの URL
- 応答の状態と転送量
- アクセス元の IP アドレス
- User-Agent（ブラウザやアプリケーションの種類）
- リファラ（どのページから来たか）

これらの記録（生ログ）は 90 日で消える。

集計では、IP アドレスから国とネットワーク事業者（ASN）を求め、日ごとの件数にまとめる。集計結果には IP アドレスそのものは残さず、異なる IP アドレスの数と、それを数えるための HyperLogLog のスケッチだけを持つ。リファラはホスト名だけを残す。

集計結果は、国土交通省向けのレポートと、運用担当者向けのレポートで見る。国土交通省向けのレポートには、IP アドレスと費用を載せない。運用担当者向けのレポートには、過剰なアクセスに対応するため、直近 90 日の生ログから作った IP アドレス別の表を載せている。国土交通省向けのレポートにも大量アクセスの状況を載せるが、IP アドレスは番号に置き換える。

## 仕組み

1. 配信サーバー（Google Cloud の外部ロードバランサと Cloud CDN）が、リクエストごとのログを Cloud Logging に書く。ログは 90 日保持する。
2. 毎日 10:30（日本時間）に、前日 1 日分のログを BigQuery で集計し、集計テーブルに書く（[bigquery/](bigquery/)）。
3. Looker Studio のレポートが集計テーブルを読む（[looker-studio/](looker-studio/)）。

配信されるファイルの URL（`/assets/<uuid[0:2]>/<uuid[2:]>/...`）からは、どの都市・年度・データ種別のファイルかが分からない。そこで、ファイルとデータセットの対応表（アセットインデックス）をデータカタログから毎日書き出し、URL の UUID で結びつける。書き出し方は [server/docs/plateau-view-data-flow.md](../../server/docs/plateau-view-data-flow.md) の「アセットインデックスの出力」にある。

- 日付はすべて日本時間（JST）で区切る。
- ログの取り込み、集計の定期実行、テーブルとビューは、Google Cloud の設定としてこのリポジトリの外で管理している。

## テーブルとビュー

BigQuery のプロジェクト `reearth-plateau`、データセット `cms_assets_analytics` にある。

| 名前 | 種類 | 内容 |
|---|---|---|
| `daily_usage` | テーブル | 日 × ファイル × リファラ × User-Agent × 国 × ASN × キャッシュの状態ごとの、リクエスト数・アクセス数・ダウンロード数・転送量 |
| `daily_clients` | テーブル | 日 × リファラ × User-Agent × 国 × ASN ごとの、`daily_usage` と同じ件数と、IP アドレスを数えるためのスケッチ（ファイルの区別なし） |
| `daily_asset_clients` | テーブル | 日 × ファイル × リファラ × User-Agent × 国 × ASN ごとの、`daily_usage` と同じ件数と、IP アドレスを数えるためのスケッチ（キャッシュの状態の区別なし） |
| `asset_index_daily` | テーブル | アセットインデックスの日ごとの写し |
| `asset_dim` | ビュー | ファイルごとのデータセット。データカタログから外れたファイルも、最後に載っていたときのデータセットを残す |
| `daily_cost` | ビュー | `daily_usage` に、通信の種類（利用者・CDN・配信側のサーバ）、アクセス元の分類、推定費用を付けたもの |
| `unit_price` / `country_region` | ビュー | 推定費用に使う単価と、国から料金区分への対応 |
| `ipv4_country` / `ipv4_asn` | テーブル | IP アドレスから国と ASN を引く表 |
| `storage_by_asset` / `storage_cost` | テーブル / ビュー | ファイルごとの保存容量と保存費用（週 1 回） |

集計テーブルには、User-Agent やリファラのホスト名をそのまま持たせ、アクセス元の分類はビュー（`daily_cost`）で行う。分類の決まりを変えても、過去の日を集計し直さずに済む。

## 集計の SQL

[bigquery/](bigquery/) に、日ごとの集計とビューの SQL を置く。実際に動いているのは Google Cloud の設定側にある同じ SQL で、ここにあるのはその写しである。

| ファイル | 使われ方 |
|---|---|
| [cms_assets_daily.sql.tftpl](bigquery/cms_assets_daily.sql.tftpl) | 毎日 10:30（日本時間）に、前日分を `daily_usage`・`daily_clients`・`daily_asset_clients` に集計し、`asset_index_daily` に写しを取る |
| [cms_assets_storage.sql.tftpl](bigquery/cms_assets_storage.sql.tftpl) | 週 1 回、保存されているファイルの一覧をファイルごとに合計して `storage_by_asset` に書く |
| [cms_assets_daily_cost.sql.tftpl](bigquery/cms_assets_daily_cost.sql.tftpl) | ビュー `daily_cost` |
| [cms_assets_asset_dim.sql.tftpl](bigquery/cms_assets_asset_dim.sql.tftpl) | ビュー `asset_dim` |
| [cms_assets_storage_cost.sql.tftpl](bigquery/cms_assets_storage_cost.sql.tftpl) | ビュー `storage_cost` |
| [cms_assets_country_region.sql](bigquery/cms_assets_country_region.sql) | ビュー `country_region` |

`.tftpl` のファイルは、次の値を埋めてから実行する。

- `${dataset}`：集計結果のデータセット（`reearth-plateau.cms_assets_analytics`）
- `${logs_dataset}`：生ログのデータセット（`reearth-plateau.cms_assets_requests_analytics`）
- `@run_date`：集計を実行する日。実行は日本時間の 0 時より後なので、日本時間の日付と同じになる。この前日の分を集計する

ビュー `unit_price` の SQL はここに置かない。[scripts/unit_price.py](scripts/unit_price.py) で、Cloud Billing の請求データから月ごとの平均単価を求めて作る。

```bash
uv run analytics/cms-assets/scripts/unit_price.py \
  --billing-table <請求データのエクスポート先テーブル> --project reearth-plateau --month 2026-09 \
  > cms_assets_unit_price.sql
```

## 数えるもの

レポートでは、利用者がデータを取得した量を次の 2 つで表す。

- **アクセス数**：取得に成功したリクエスト（GET で、応答が 200・206・304）の数。
  - 大きなファイルを分割して取得したリクエスト（206）は、同じ IP アドレス・ファイル・日で 1 回と数える。分割の細かさはアプリケーションによって違い、1 つのファイルが数千件のリクエストになることもあるため。
  - 失敗したリクエスト（404 など）は数えない。
- **ダウンロード数**：実際にデータを送った（応答が 200・206）ファイルの数。同じ日に同じ IP アドレスへ同じファイルを何度送っても 1 回と数える。

次の通信は、どちらにも、IP アドレス数にも、推定費用にも含めない（`daily_cost` の `traffic_class` が `client` の行だけを使う）。

- **CDN 自身の確認のためのリクエスト**。User-Agent が `Cloud-CDN-Google` で始まる記録で、CDN が保存しているファイルが最新かを確かめたもの。元の利用者の IP アドレスで記録されるので、除かないと利用者のアクセスを二重に数えることになる。
- **PLATEAU の配信サーバ自身の通信**。タイルや API のサーバが配信データを読み込む通信で、Google Cloud のネットワーク（AS396982・AS15169）から、User-Agent なし・`Go-http-client/`・`object_store/` で届く。同じネットワークを使う第三者の通信は、User-Agent が違うので利用者として数える。

## 数え方の注意

- **IP アドレス数は、行を足すと重なる。** レポートの IP アドレス数は、行・日ごとに重なりなく数えた数を足したもの（延べ数）である。複数の行や日に現れる IP アドレスは、それぞれで数える。利用者の人数ではない。
  - 行・日ごとの数は、`daily_clients` と `daily_asset_clients` のスケッチ（`ip_sketch`）を `HLL_COUNT.MERGE` でまとめて求める。行をまとめても重ならないが、1% 未満の誤差がある。
  - `daily_usage` の `ips` は、その行の中だけで数えた数なので、足すと実際の数倍になる。
- **前日分の集計には、わずかな漏れがある。** ログは最大 24 時間ほど遅れて届くことがあり、リクエストの 0.02% ほどが集計に入らない。
- **集計は 2026年10月13日から。** ログの記録は 10月12日の 14:40（日本時間）に始まったので、12日は 1 日分そろっていない。
- **1 つのファイルが複数のデータセットに属することがある。** 合計する前に 1 つに絞る（`asset_dim` を使う）。

## 推定費用

`daily_cost` の費用は、使用量に単価を掛けた推定値（円）である。実際の請求額ではない。単価には、ある 1 か月の請求から求めた平均を固定で使う。

| 列 | 対象 |
|---|---|
| `cdn_egress_jpy` | CDN を通った転送量 × CDN の配信単価（地域別） |
| `internet_egress_jpy` | CDN を通らない転送量 × インターネットへの転送単価（地域別）。CDN は 1 ファイル 100 GiB までしか保存しないため、それより大きいファイルはこちらになる |
| `cache_fill_jpy` | CDN が元のストレージから取り寄せた量 × 取り寄せの単価 |
| `cache_lookup_jpy` | CDN の検索回数 × 検索の単価 |
| `gcs_ops_jpy` | CDN になかったリクエスト数 × ストレージの読み出し単価 |

推定費用の合計は、この 5 列の和である。

中国のアクセス元は、中国向けではなくアジア向けの単価で計算する。2026年10月の請求では、中国の IP アドレスへの転送のうち中国向けの料金になっていたのは 3% ほどだったため。

## レポートの SQL

[looker-studio/](looker-studio/) に、Looker Studio のレポートが集計テーブルを読むための SQL を置く。

| ファイル | レポートとページ | 期間 |
|---|---|---|
| [mlit-sources.sql](looker-studio/mlit-sources.sql) | 国土交通省向け「アクセス元」 | 全期間 |
| [mlit-assets.sql](looker-studio/mlit-assets.sql) | 国土交通省向け「自治体・データ別」 | 直近 180 日 |
| [internal-sources.sql](looker-studio/internal-sources.sql) | 運用担当者向け「アクセス元」 | 全期間 |
| [internal-ips.sql](looker-studio/internal-ips.sql) | 運用担当者向け「アクセス元」の IP アドレス別の表 | 直近 90 日 |
| [internal-assets.sql](looker-studio/internal-assets.sql) | 運用担当者向け「自治体・データ別」 | 全期間 |
| [mlit-heavy-users.sql](looker-studio/mlit-heavy-users.sql) | 国土交通省向け「大量アクセス」の利用者別の表 | 直近 90 日 |
| [mlit-heavy-weekly.sql](looker-studio/mlit-heavy-weekly.sql) | 国土交通省向け「大量アクセス」の週ごとの分布 | 直近 90 日 |
| [internal-heavy-users.sql](looker-studio/internal-heavy-users.sql) | 運用担当者向け「大量アクセス」の利用者別の表 | 直近 90 日 |
| [internal-heavy-weekly.sql](looker-studio/internal-heavy-weekly.sql) | 運用担当者向け「大量アクセス」の週ごとの分布 | 直近 90 日 |

- 国・リファラ・ASN・User-Agent・IP アドレスは件数の多いものだけを残し、残りは「(その他)」にまとめる。
- 国土交通省向け「自治体・データ別」は、レポートが一度に取り込めるデータ量の上限があるため、直近 180 日に絞っている。
- 「大量アクセス」のページは、IP アドレスごとの量を見るため生ログから作る。
  - 週ごとの分布は、アクセスが 1 回以上あった IP アドレスを 1 週間（月曜始まり）のアクセス数と転送量で分け、それぞれの件数と割合を出す。日曜日まで終わった週だけを出す。
  - 利用者別の表は、直近 90 日のアクセス数・転送量（運用担当者向けは推定費用も）のいずれかで上位 300 件に入る IP アドレスを個別に出し、残りは「(上位以外の合計)」の 1 行にまとめる。
  - 国土交通省向けでは、IP アドレスの代わりに「利用者0001」のような番号を出す。番号は直近 90 日の転送量の多い順で、毎日振り直すので、IP アドレスとは結びつかない。

## 帰属表示

IP アドレスから国とネットワーク事業者を求めるのに、[DB-IP](https://db-ip.com) の IP to Country Lite と IP to ASN Lite（[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)）を使っている。

`ipv4_country` と `ipv4_asn` は、DB-IP が月ごとに公開している `dbip-country-lite-YYYY-MM.csv` と `dbip-asn-lite-YYYY-MM.csv` から作る。IPv4 の範囲だけを取り、検索を速くするため、範囲がまたがる /16 ごとに行を分けて `bucket = DIV(IP アドレスの整数値, 65536)` を持たせる。
