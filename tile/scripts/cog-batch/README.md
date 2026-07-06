# cog-batch — 原本TIFF → 配信用COG バッチ変換

原本バックアップ（R2 `plateau-terrain-ortho-backup`）から、`/tile` 用の配信 COG を作って
`plateau-terrain` / `plateau-ortho` に書き出す。**GCE Spot VM で回す**想定（Mac はディスク/帯域が足枷）。
レシピは能登 PoC で実機確定済み（`../../README.md`「Choosing the CRS」/ `backup-listings/COG-candidates.md`）。

## 構成
- `provision-vm.sh` … GCE Spot VM 作成（GDAL+rclone 自動セットアップ・balanced PD scratch）
- `convert-cogs.sh` … 変換ドライバ（DL→変換→アップ→削除を各グループで回す＝ディスク一定）
- `rclone.conf.example` … src(読取)/dst(書込) の 2 remote

## レシピ（種別ごと）
| 種別 | 元CRS | 出力 | gdal_translate -of COG の主オプション |
|---|---|---|---|
| ベースDEM dem10/5/1 | 4326(WGS84) | geographic キープ | `RESAMPLING=nearest COMPRESS=ZSTD PREDICTOR=3` |
| パッチ 能登/静岡/東京 | 平面直角6675/76/77 | **3857** | `TILING_SCHEME=GoogleMapsCompatible BLOCKSIZE=256 RESAMPLING=nearest ZSTD PREDICTOR=3` |
| パッチ 他都市 | 6668(地理) | geographic キープ | DEM geo と同じ |
| オルソ | 3857 | 3857 キープ | `TILING_SCHEME=GoogleMapsCompatible RESAMPLING=cubic COMPRESS=WEBP QUALITY=85` |

- DEM は必ず **nearest**（nodata ブレンドのスパイク回避）。nodata=-9999。
- パッチは**標高(orthometric)ステージのみ**使用（東京/能登=`s1_geotiff_raw`, 静岡=`s2_geotiff`, 他都市=各source）。楕円体高版は使わない。

## 出力キー配置
```
plateau-terrain/base/{dem10,dem5,dem1}/<1次メッシュ4桁>.tif
plateau-terrain/patch/{noto/<region>, tokyo/<23ku|tama>, shizuoka/<area>, others/<city>}.tif
plateau-ortho/<year>/<city>.tif
```
ベースDEMは 1次メッシュ(FG-GML-XXXX)単位で分割 → R*-tree footprint が適度な粒度に
（dem5a=28.7万枚を1枚COGにしない。dem1 は1枚が大きめ→必要なら2次メッシュに細分可）。

## 実行
```bash
# 0) 事前: dstバケット作成 + S3書込トークン
npx wrangler r2 bucket create plateau-terrain
npx wrangler r2 bucket create plateau-ortho
cp rclone.conf.example rclone.conf   # 値を埋める（dstは書込権限必須）

# 1) VM
bash provision-vm.sh
gcloud compute scp convert-cogs.sh rclone.conf cog-batch:~ --zone=asia-northeast1-b
gcloud compute ssh cog-batch --zone=asia-northeast1-b

# 2) VM上（まず小さく検証 → 全部）
export RC="rclone --config ~/rclone.conf --transfers=32 --checkers=64 --s3-no-check-bucket"
sudo WORK=/scratch RC="$RC" bash convert-cogs.sh patch noto      # 検証
sudo WORK=/scratch RC="$RC" J=$(nproc) ORTHO_J=12 bash convert-cogs.sh all

# 3) 破棄
gcloud compute instances delete cog-batch --zone=asia-northeast1-b
```

## Spot preemption 耐性
Spot VM は途中で回収され得る。本スクリプトは次で耐える：
- **冪等**: 各グループの出力COGが dst に既に有れば**スキップ**（`dst_exists` を `rclone lsf` で確認）。
  → 何度でも再実行でき、完了済みは飛ばして**続きから再開**。強制再作成は `FORCE=1`。
- **継続**: `set -e` を使わず、1グループ/1タイルの失敗で全体を止めない（失敗は再実行で拾い直す）。
- **resume は dst 存在ベース**（scratch は VM 消滅で失われる前提）。ベースDEMは開始時に mesh 一覧を
  `lsf` で取り、**未完了メッシュだけDL→変換**（進行中の解像度のみ再DL、egress無料）。オルソ/パッチは
  元から都市/地域単位なので粒度が細かく preempt 向き。

VM 自体の**自動再起動**は 2 択：
1. **MIG（Managed Instance Group, Spot）** … サイズ1・autohealing。preempt→MIGが再作成→startup-script が
   `convert-cogs.sh all` を再実行→冪等なので再開。テンプレに rclone.conf と本スクリプトを metadata/GCS で渡す。
2. **Cloud Batch** … Spot task の**自動リトライがネイティブ**。1グループ=1 task に割ると最も堅い
   （task 粒度で並列＆再試行）。大規模一括はこちらが本命。

`provision-vm.sh` は単発VM（＝手動再実行で resume）。無人で回すなら MIG か Cloud Batch へ。

## 見積り（PoC 0.46倍基準・要精緻化）
DEM ~90-105 GiB / オルソ ~60-120 GiB → **総計 ~150-230 GiB**（R2 保存 ≈ $2-3.5/月）。

## 未確定 / 要決定（TODO）
- [ ] **R2 書込トークン**（plateau-terrain/plateau-ortho）発行 → 現行 backup 用トークンは書込 403。
- [ ] **静岡 `s2_geotiff` が標高か**の最終確認（代表タイル `gdalinfo`。s4 が楕円体高なのは確認済）。
- [ ] **他都市**の生DEM tif の実パス/CRS/標高 or 楕円体高（`03_.../<city>` 配下を要 gdalinfo）。
- [ ] ベースDEM のメッシュ分割粒度（dem1 は 1次メッシュだと大きめ → 2次に細分するか）。
- [ ] オルソ WEBP QUALITY（85 で良いか）＆ nodata/alpha 有無（黒縁の透過処理）。
- [ ] マシンサイズ / ортho 並列度（ディスク peak = ORTHO_J × 都市サイズ）。
- [ ] 変換後の `/tile` config(JSON) 自動生成（COG 一覧 → dem overlays 配列）。
