-- PLATEAU CMS asset access per day (JST) by municipality and dataset, last 180 days. For MLIT: no cost, cache
-- details or IP addresses. Counts client traffic only (traffic_class = "client"): not the CDN's own
-- revalidation requests and not our own servers. requests is the number of accesses (successful GETs;
-- the 206 parts of a file count once per IP and day), downloads the number of files sent.
-- ip_days counts distinct IPs per day and output row from the HLL sketches in daily_asset_clients;
-- summed over rows or days it is a total of IP-days.
WITH usage AS (
  SELECT
    day, uuid, file, referer_host, user_agent, country, asn,
    SUM(accesses) AS requests, SUM(downloads) AS downloads, SUM(bytes) AS bytes
  FROM `reearth-plateau.cms_assets_analytics.daily_cost`
  -- The last 180 days keep the report's data extract under its 100 MB limit. Logs start on
  -- 2026-10-12 at 14:40 JST; earlier days are incomplete.
  WHERE traffic_class = "client"
    AND day >= GREATEST(DATE "2026-10-13", DATE_SUB(CURRENT_DATE("Asia/Tokyo"), INTERVAL 180 DAY))
  GROUP BY day, uuid, file, referer_host, user_agent, country, asn
),
joined AS (
  SELECT
    u.day, u.uuid, u.file, u.country, u.requests, u.downloads, u.bytes, c.ip_sketch
  FROM usage u
  LEFT JOIN `reearth-plateau.cms_assets_analytics.daily_asset_clients` c
    ON c.day = u.day
    AND IFNULL(c.uuid, "") = IFNULL(u.uuid, "")
    AND IFNULL(c.file, "") = IFNULL(u.file, "")
    AND IFNULL(c.referer_host, "") = IFNULL(u.referer_host, "")
    AND IFNULL(c.user_agent, "") = IFNULL(u.user_agent, "")
    AND c.country = u.country
    AND IFNULL(c.asn, -1) = IFNULL(u.asn, -1)
),
idx AS (SELECT uuid, refs FROM `reearth-plateau.cms_assets_analytics.asset_index`),
latest AS (SELECT MAX(snapshot_date) AS d FROM `reearth-plateau.cms_assets_analytics.asset_index_daily`),
-- Names for labeling assets that are no longer in the catalog by their file names.
code_names AS (
  SELECT code, ANY_VALUE(pref) AS pref, ANY_VALUE(city) AS city, ANY_VALUE(ward) AS ward FROM (
    SELECT r.city_code AS code, r.pref, r.city, CAST(NULL AS STRING) AS ward FROM idx, UNNEST(refs) r WHERE r.city_code IS NOT NULL
    UNION ALL
    SELECT r.ward_code, r.pref, r.city, r.ward FROM idx, UNNEST(refs) r WHERE r.ward_code IS NOT NULL
  ) GROUP BY code
),
pref_names AS (SELECT r.pref_code, ANY_VALUE(r.pref) AS pref FROM idx, UNNEST(refs) r WHERE r.pref_code IS NOT NULL GROUP BY 1),
type_names AS (SELECT r.type_en, ANY_VALUE(r.type) AS type FROM idx, UNNEST(refs) r WHERE r.type_en IS NOT NULL GROUP BY 1),
labeled AS (
  SELECT
    j.*,
    -- In the catalog when the asset is in the latest asset index snapshot. asset_dim keeps the
    -- catalog entry of assets that have since left the catalog, so they keep their labels.
    CASE
      WHEN j.uuid IS NULL THEN "アセット以外"
      WHEN d.last_seen = (SELECT d FROM latest) THEN "掲載中"
      ELSE "カタログ外"
    END AS catalog,
    d.ref AS ref,
    REGEXP_EXTRACT(j.file, r"^(\d{5})_") AS f_code,
    SAFE_CAST(REGEXP_EXTRACT(j.file, r"^\d{5}_[a-z0-9-]+?_(?:(?:city|pref)_)?(\d{4})_") AS INT64) AS f_year,
    REGEXP_EXTRACT(j.file, r"_op_([a-z]+)(?:_|$)") AS f_feature,
    REGEXP_EXTRACT(j.file, r"^\d{5}_([a-z0-9-]+?)_") AS f_slug
  FROM joined j
  LEFT JOIN `reearth-plateau.cms_assets_analytics.asset_dim` d USING (uuid)
),
dims AS (
  SELECT
    l.*,
    COALESCE(l.ref.pref, cn.pref, pn.pref, IF(REGEXP_CONTAINS(l.file, r"ortho"), "(広域)", NULL), "(不明)") AS pref,
    COALESCE(l.ref.city, cn.city, IF(l.f_code = "13100", "23区", l.f_slug), "(不明)") AS city,
    COALESCE(l.ref.ward, cn.ward) AS ward,
    COALESCE(l.ref.city_code, l.f_code) AS city_code,
    COALESCE(l.ref.year, l.f_year) AS year,
    CASE
      WHEN l.ref.type IS NOT NULL THEN l.ref.type
      WHEN l.ref.kind = "citygml" THEN "CityGML"
      WHEN l.ref.kind LIKE "citygml_%" THEN "CityGML（付属）"
      WHEN l.ref.kind LIKE "gspatialjp_%" THEN "G空間情報センター向けzip"
      WHEN l.catalog = "アセット以外" THEN "(アセット以外)"
      WHEN tn.type IS NOT NULL THEN tn.type
      WHEN REGEXP_CONTAINS(l.file, r"ortho") THEN "オルソ画像"
      WHEN REGEXP_CONTAINS(l.file, r"^template_objectlist") THEN "属性名称対応表（CSV）"
      WHEN REGEXP_CONTAINS(l.file, r"3dtiles[-_]mvt") THEN "3D Tiles・MVT（一括zip）"
      WHEN REGEXP_CONTAINS(l.file, r"_citygml") THEN "CityGML"
      ELSE "(不明)"
    END AS data_type,
    l.ref.lod AS lod,
    IF(l.catalog = "カタログ外", l.file, NULL) AS out_file,
    IF(l.catalog = "カタログ外", CONCAT("https://assets.cms.plateau.reearth.io/assets/", l.uuid, "/", l.file), NULL) AS asset_url
  FROM labeled l
  LEFT JOIN code_names cn ON cn.code = l.f_code
  LEFT JOIN pref_names pn ON pn.pref_code = SUBSTR(l.f_code, 1, 2)
  LEFT JOIN type_names tn ON tn.type_en = l.f_feature
),
top_country AS (
  SELECT country FROM dims GROUP BY country
  QUALIFY ROW_NUMBER() OVER (ORDER BY SUM(requests) DESC) <= 25
)
SELECT
  d.day, d.catalog, d.pref, d.city, d.ward, d.city_code, d.year, d.data_type, d.lod,
  d.out_file AS file, d.asset_url,
  IF(tc.country IS NULL, "(その他)", d.country) AS country,
  SUM(d.requests) AS requests,
  SUM(d.downloads) AS downloads,
  IFNULL(HLL_COUNT.MERGE(d.ip_sketch), 0) AS ip_days,
  SUM(d.bytes) / POW(1024, 3) AS gib
FROM dims d
LEFT JOIN top_country tc USING (country)
GROUP BY day, catalog, pref, city, ward, city_code, year, data_type, lod, file, asset_url, country
