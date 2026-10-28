-- PLATEAU CMS asset access per day (JST) by where it came from. For MLIT:
-- source (VIEW / SDK / G空間), client type, referer and network operator (ASN). No cost or IP addresses.
-- Counts client traffic only (traffic_class = "client"): not the CDN's own revalidation requests and
-- not our own servers. requests is the number of accesses (successful GETs; the 206 parts of a file
-- count once per IP and day), downloads the number of files sent (distinct IP and path per day).
-- ips counts distinct IPs per day and output row from the HLL sketches in daily_clients, so it does
-- not overlap within a row; summed over rows or days it is a total of IP-days.
WITH usage AS (
  SELECT
    day, referer_host, user_agent, country, asn, client_type, source, asn_org,
    SUM(accesses) AS accesses, SUM(downloads) AS downloads, SUM(bytes) AS bytes
  FROM `reearth-plateau.cms_assets_analytics.daily_cost`
  -- Logs start on 2026-10-12 at 14:40 JST; earlier days are incomplete.
  WHERE traffic_class = "client" AND day >= DATE "2026-10-13"
  GROUP BY day, referer_host, user_agent, country, asn, client_type, source, asn_org
),
labeled AS (
  SELECT
    u.day,
    u.country,
    CASE u.client_type
      WHEN "none" THEN "(なし)"
      WHEN "bot" THEN "ボット・クローラ"
      WHEN "headless-browser" THEN "自動操作のブラウザ"
      WHEN "curl/wget" THEN "curl・wget"
      WHEN "python" THEN "Python"
      WHEN "go" THEN "Go"
      WHEN "other-script" THEN "その他のプログラム"
      WHEN "qgis" THEN "QGIS"
      WHEN "plateau-sdk" THEN "PLATEAU SDK"
      WHEN "3d-engine" THEN "3D エンジン（Unity・Unreal・Cesium）"
      WHEN "browser" THEN "ブラウザ"
      ELSE "その他"
    END AS client_type,
    CASE u.source
      WHEN "PLATEAU SDK" THEN "PLATEAU SDK"
      WHEN "PLATEAU VIEW" THEN "PLATEAU VIEW"
      WHEN "G-Spatial Information Center" THEN "G空間情報センター"
      WHEN "PLATEAU service" THEN "PLATEAU の関連サービス"
      WHEN "MLIT site" THEN "国土交通省のサイト"
      WHEN "local" THEN "ローカル（自分のPC・社内ネットワーク）"
      WHEN "none" THEN "(なし)"
      ELSE "その他のサイト"
    END AS source,
    IFNULL(u.referer_host, "(なし)") AS referer_host,
    -- Network operators by DB-IP (https://db-ip.com), CC BY 4.0.
    IF(u.asn IS NULL, "(不明)", CONCAT("AS", u.asn, " ", u.asn_org)) AS asn_org,
    u.accesses,
    u.downloads,
    u.bytes,
    c.ip_sketch
  FROM usage u
  LEFT JOIN `reearth-plateau.cms_assets_analytics.daily_clients` c
    ON c.day = u.day
    AND IFNULL(c.referer_host, "") = IFNULL(u.referer_host, "")
    AND IFNULL(c.user_agent, "") = IFNULL(u.user_agent, "")
    AND c.country = u.country
    AND IFNULL(c.asn, -1) = IFNULL(u.asn, -1)
),
top_country AS (
  SELECT country FROM labeled GROUP BY country
  QUALIFY ROW_NUMBER() OVER (ORDER BY SUM(accesses) DESC) <= 25
),
top_referer AS (
  SELECT referer_host FROM labeled GROUP BY referer_host
  QUALIFY ROW_NUMBER() OVER (ORDER BY SUM(accesses) DESC) <= 50
),
top_asn AS (
  SELECT asn_org FROM labeled GROUP BY asn_org
  QUALIFY ROW_NUMBER() OVER (ORDER BY SUM(bytes) DESC) <= 50
)
SELECT
  l.day,
  IF(tc.country IS NULL, "(その他)", l.country) AS country,
  l.client_type,
  l.source,
  IF(tr.referer_host IS NULL, "(その他)", l.referer_host) AS referer_host,
  IF(ta.asn_org IS NULL, "(その他)", l.asn_org) AS asn_org,
  SUM(l.accesses) AS requests,
  SUM(l.downloads) AS downloads,
  IFNULL(HLL_COUNT.MERGE(l.ip_sketch), 0) AS ips,
  SUM(l.bytes) / POW(1024, 3) AS gib
FROM labeled l
LEFT JOIN top_country tc USING (country)
LEFT JOIN top_referer tr USING (referer_host)
LEFT JOIN top_asn ta USING (asn_org)
GROUP BY day, country, client_type, source, referer_host, asn_org
