#!/usr/bin/env bash
# COG batch converter — 原本TIFF(R2) → 配信用COG(R2)。
# GCE Spot VM 等で実行（GDAL + rclone 必要）。レシピは Noto PoC で実機確定。
# 詳細: tile/scripts/cog-batch/README.md
#
# ★ Spot preemption 耐性:
#   - 冪等: 出力COGが dst に既に有るグループはスキップ（FORCE=1 で強制再作成）。
#   - 継続: 1グループの失敗で全体を止めない（失敗は再実行で拾い直す）。
#   - resume は dst の存在ベース（scratchはVM消滅で失われる前提）。
#   - VM自動再作成は MIG or Cloud Batch 側で（provision-vm.sh / README 参照）。
#     再作成VMが本スクリプトを再実行 → 冪等なので続きから再開。
set -uo pipefail                      # ※ -e は付けない（1グループ失敗で全滅させない）

: "${WORK:=/scratch}"
: "${J:=$(nproc)}"                    # DEMメッシュ並列
: "${ORTHO_J:=8}"                     # オルソ都市並列（1都市が大きいので控えめ）
: "${FORCE:=0}"                       # 1で既存COGも再作成
SRC="${SRC:-src:plateau-terrain-ortho-backup}"
DST_T="${DST_T:-dst:plateau-terrain}"
DST_O="${DST_O:-dst:plateau-ortho}"
RC="${RC:-rclone --transfers=32 --checkers=64 --s3-no-check-bucket}"
GDAL_OPTS=(-co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES)
export SRC DST_T DST_O RC WORK FORCE
BASE_SRC="terrain/base_terrain/kibanchizu_dem_20250129/s1_geotiff_raw"
PATCH_SRC="terrain/plateau-terrain_2024/dem_for_overwriting"
ORTHO_SRC="ortho/20250220_plateau_ortho_renewal/tileset_for_each_city"
export BASE_SRC PATCH_SRC ORTHO_SRC GDAL_OPTS_STR="${GDAL_OPTS[*]}"

log(){ echo "[$(date -u +%H:%M:%S)] $*" >&2; }
# 出力が既に存在？（冪等スキップ用）。$1 = dst のフルパス(ファイル名込み)
dst_exists(){ [ "$FORCE" = 1 ] && return 1; [ -n "$($RC lsf "$1" 2>/dev/null)" ]; }
export -f log dst_exists

# --- COGレシピ（PoC確定） ---
cog_dem_geo(){  gdal_translate -of COG "$1" "$2" -co RESAMPLING=nearest \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES; }
cog_dem_3857(){ gdal_translate -of COG "$1" "$2" -co TILING_SCHEME=GoogleMapsCompatible \
  -co BLOCKSIZE=256 -co ZOOM_LEVEL_STRATEGY=AUTO -co RESAMPLING=nearest \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES; }
cog_ortho(){    gdal_translate -of COG "$1" "$2" -co TILING_SCHEME=GoogleMapsCompatible \
  -co BLOCKSIZE=256 -co ZOOM_LEVEL_STRATEGY=AUTO -co RESAMPLING=cubic \
  -co COMPRESS=WEBP -co QUALITY=85 -co OVERVIEW_QUALITY=85 -co NUM_THREADS=ALL_CPUS -co BIGTIFF=YES; }
export -f cog_dem_geo cog_dem_3857 cog_ortho

# 1グループ処理を「失敗しても戻り値で返す」小さな単位に（set -e無し環境なので明示チェック）
# ============================================================
# ベースDEM: 解像度ごとにDL → 1次メッシュ単位でCOG化（冪等スキップ付き）
# ============================================================
_base_one_mesh(){                     # $1=res $2=mesh4
  local res="$1" mesh="$2"; local d="$WORK/$res"   # ★dは別local（同一local内だと前の変数が空になる）
  local dst="$DST_T/base/$res/${mesh}.tif"
  if dst_exists "$dst"; then log "  base/$res/$mesh SKIP(exists)"; return 0; fi
  find "$d" -name "FG-GML-${mesh}-*.tif" > "$d/$mesh.list" || return 1
  gdalbuildvrt -srcnodata -9999 -vrtnodata -9999 -input_file_list "$d/$mesh.list" "$d/$mesh.vrt" >/dev/null 2>&1 || { log "  base/$res/$mesh VRT FAIL"; return 1; }
  cog_dem_geo "$d/$mesh.vrt" "$d/$mesh.tif" || { log "  base/$res/$mesh COG FAIL"; return 1; }
  $RC copyto "$d/$mesh.tif" "$dst" || { log "  base/$res/$mesh UPLOAD FAIL"; return 1; }
  rm -f "$d/$mesh.tif" "$d/$mesh.vrt" "$d/$mesh.list"; log "  base/$res/$mesh OK"
}
export -f _base_one_mesh

convert_base(){                       # $1=dem10|dem5|dem1
  local res="$1"; local -a subs
  case "$res" in
    dem10) subs=(dem10a dem10b);; dem5) subs=(dem5a dem5b dem5c);; dem1) subs=(dem1a);;
    *) log "unknown res $res"; return 1;;
  esac
  local d="$WORK/$res"; mkdir -p "$d"
  # 先に mesh 一覧を軽量取得（lsf）し、全メッシュ完了なら DL 自体スキップ（preempt再開の高速化）
  log "base/$res: list source meshes"
  : > "$d/meshes.src"
  for s in "${subs[@]}"; do $RC lsf "$SRC/$BASE_SRC/$s" --include '*.tif' 2>/dev/null; done \
    | sed -E 's/^FG-GML-([0-9]{4}).*/\1/' | sort -u > "$d/meshes.src"
  # 未完了メッシュだけ抽出
  : > "$d/meshes.todo"
  while read -r m; do dst_exists "$DST_T/base/$res/${m}.tif" || echo "$m"; done < "$d/meshes.src" > "$d/meshes.todo"
  local todo; todo=$(wc -l < "$d/meshes.todo" | tr -d ' ')
  if [ "$todo" -eq 0 ]; then log "base/$res: all $(wc -l < "$d/meshes.src") meshes already done, skip"; rm -rf "$d"; return 0; fi
  log "base/$res: download ${subs[*]} ($todo/$(wc -l < "$d/meshes.src") meshes todo)"
  for s in "${subs[@]}"; do $RC copy "$SRC/$BASE_SRC/$s" "$d"; done
  log "base/$res: convert J=$J"
  xargs -P "$J" -I{} bash -c '_base_one_mesh "$1" "$2"' _ "$res" {} < "$d/meshes.todo"
  rm -rf "$d"; log "base/$res: DONE"
}

# ============================================================
# パッチ: 標高ステージのみ。平面直角→3857 / 地理→geo。（冪等スキップ付き）
# ============================================================
_patch_region(){                      # $1=out(キー) $2=srcpath  (NoData/CRSは自動検出)
  local out="$1" src="$2"; local d="$WORK/patch_$(printf '%s' "$out" | tr / _)"
  local dst="$DST_T/patch/${out}.tif"
  if dst_exists "$dst"; then log "  patch/$out SKIP(exists)"; return 0; fi
  mkdir -p "$d"
  $RC copy "$SRC/$src" "$d" || { log "  patch/$out DL FAIL"; rm -rf "$d"; return 1; }
  find "$d" \( -name '*.tif' -o -name '*.asc' \) > "$d/files.txt"   # .asc(ESRI ASCII Grid, .prj同梱)も可。ファイル渡しでARG_MAX回避
  local sample; sample=$(head -1 "$d/files.txt")
  [ -z "$sample" ] && { log "  patch/$out no tif, skip"; rm -rf "$d"; return 0; }
  local n; n=$(wc -l < "$d/files.txt" | tr -d ' ')
  # NoData と EPSG をソースから自動検出（市ごとに違うため決め打ちしない）
  local nd epsg; local ndopt=()
  nd=$(gdalinfo "$sample" 2>/dev/null | sed -n 's/^ *NoData Value=//p' | head -1)
  [ -n "$nd" ] && ndopt=(-srcnodata "$nd" -vrtnodata "$nd")
  epsg=$(gdalsrsinfo -o epsg "$sample" 2>/dev/null | grep -oE '[0-9]{4,5}' | tail -1)
  gdalbuildvrt "${ndopt[@]}" -input_file_list "$d/files.txt" "$d/m.vrt" >/dev/null 2>&1 || { log "  patch/$out VRT FAIL"; rm -rf "$d"; return 1; }
  case "$epsg" in
    4326|6668|4612|4301) log "  patch/$out epsg=$epsg geo nd=${nd:-none} ($n)"; cog_dem_geo "$d/m.vrt" "$d/out.tif";;
    *)                   log "  patch/$out epsg=${epsg:-?} ->3857 nd=${nd:-none} ($n)"; cog_dem_3857 "$d/m.vrt" "$d/out.tif";;
  esac || { log "  patch/$out COG FAIL"; rm -rf "$d"; return 1; }
  $RC copyto "$d/out.tif" "$dst" || { log "  patch/$out UPLOAD FAIL"; rm -rf "$d"; return 1; }
  rm -rf "$d"; log "  patch/$out OK"
}

convert_patch(){                      # $1=noto|tokyo|shizuoka|others|all
  local which="${1:-all}"
  if [ "$which" = noto ] || [ "$which" = all ]; then
    : "${NOTO_REGIONS:=01_northen_noto 02_central_noto 03_southern_noto}"   # 一部だけ試す時に絞れる
    for r in $NOTO_REGIONS; do
      _patch_region "noto/${r#*_}" "$PATCH_SRC/04_noto_dem_from_aas/s1_geotiff_raw/full_resolution/$r"
    done
  fi
  if [ "$which" = tokyo ] || [ "$which" = all ]; then
    for r in 23ku tama; do
      _patch_region "tokyo/$r" "$PATCH_SRC/01_tokyo_dem_from_psc/s1_geotiff_raw/$r"      # 6677→3857(自動)
    done
  fi
  if [ "$which" = shizuoka ] || [ "$which" = all ]; then
    local sbase="$PATCH_SRC/02_shizuoka_dem_from_g-center_20250304/s2_geotiff/full_resolution"  # s2=標高(s4は楕円体高で不可)
    $RC lsf --dirs-only "$SRC/$sbase" 2>/dev/null | while read -r area; do
      _patch_region "shizuoka/${area%/}" "$sbase/${area%/}"                              # 6676→3857(自動)
    done
  fi
  if [ "$which" = others ] || [ "$which" = all ]; then
    # ⚠ 市ごとにDEMの所在・CRS・NoDataが全部違う（gdalinfoで確認済）。正しいsubdirを個別指定。
    local obase="$PATCH_SRC/03_plateau2024_other_cities"
    _patch_region "others/susami-cho" "$obase/30406_susami-cho/source/30406_susami-cho_city_2024_citygml_1_dem_tif"  # 6668地理, nodata=255
    _patch_region "others/yonago-shi" "$obase/31202_yonago-shi/source/yonago_all_dem1"                                # 6673平面直角→3857
    _patch_region "others/tamana-shi" "$obase/43206_tamana-shi/s1_geotiff"                                            # 6670平面直角→3857
    _patch_region "others/kisarazu-shi" "$obase/12206_kisarazu-shi/source_from_kkc"                                   # .asc(TM/EPSG無しWKT)→3857。同dirの.zipはfindが無視
  fi
}

# ============================================================
# オルソ: 3857キープ・都市ごと（冪等スキップ・自然に粒度が細かい＝preempt向き）
# ============================================================
_ortho_city(){                        # $1=year $2=city
  local year="$1" city="$2"; local d="$WORK/ortho_${year}_${city}"   # ★dは別local（同一local内だと前の変数が空になる）
  local dst="$DST_O/${year}/${city}.tif"
  if dst_exists "$dst"; then log "  ortho/$year/$city SKIP(exists)"; return 0; fi
  mkdir -p "$d"
  $RC copy "$SRC/$ORTHO_SRC/${year}_xyz/s1_merged_3857/$city" "$d" || { log "  ortho/$year/$city DL FAIL"; rm -rf "$d"; return 1; }
  find "$d" -name '*.tif' > "$d/files.txt"
  gdalbuildvrt -input_file_list "$d/files.txt" "$d/m.vrt" >/dev/null 2>&1 || { log "  ortho/$year/$city VRT FAIL"; rm -rf "$d"; return 1; }
  cog_ortho "$d/m.vrt" "$d/out.tif" || { log "  ortho/$year/$city COG FAIL"; rm -rf "$d"; return 1; }
  $RC copyto "$d/out.tif" "$dst" || { log "  ortho/$year/$city UPLOAD FAIL"; rm -rf "$d"; return 1; }
  rm -rf "$d"; log "  ortho/$year/$city OK"
}
export -f _ortho_city _patch_region

convert_ortho(){                      # $1=year|all
  local years=("$1"); [ "${1:-all}" = all ] && years=(2020 2022 2023 2024)
  for year in "${years[@]}"; do
    log "ortho $year: list cities (ORTHO_J=$ORTHO_J${ORTHO_CITIES:+, filter=$ORTHO_CITIES})"
    # ORTHO_CITIES に空白区切りで都市名(部分一致)を指定すると一部だけ変換できる
    $RC lsf --dirs-only "$SRC/$ORTHO_SRC/${year}_xyz/s1_merged_3857" 2>/dev/null | sed 's#/$##' \
      | { if [ -n "${ORTHO_CITIES:-}" ]; then grep -E "$(echo "$ORTHO_CITIES" | tr ' ' '|')"; else cat; fi; } \
      | xargs -P "$ORTHO_J" -I{} bash -c '_ortho_city "$1" "$2"' _ "$year" {}
    log "ortho $year: DONE"
  done
}

# ---- dispatch（各グループは失敗しても次へ進む＝再実行で拾い直せる） ----
case "${1:-}" in
  base)   convert_base "${2:?res}";;
  patch)  convert_patch "${2:-all}";;
  ortho)  convert_ortho "${2:-all}";;
  all)
    for r in dem10 dem5 dem1; do convert_base "$r" || log "base/$r had failures (re-run to retry)"; done
    convert_patch all || log "patch had failures (re-run to retry)"
    convert_ortho all || log "ortho had failures (re-run to retry)"
    ;;
  *) echo "usage: $0 {base <dem10|dem5|dem1>|patch <noto|tokyo|shizuoka|others|all>|ortho <year|all>|all}  (FORCE=1 で再作成)" >&2; exit 1;;
esac
log "==== run finished: ${*}  (未完了が有れば同じコマンドを再実行すれば続きから) ===="
