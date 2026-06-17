import axios from "axios";

import { AreasFetcherBase, UPSTREAM_TIMEOUT_MS } from "./types";

export const fetchGovPolygon: AreasFetcherBase = async (url, lon, lat) => {
  const { data } = await axios.get(url, {
    params: {
      lng: lon,
      lat,
    },
    timeout: UPSTREAM_TIMEOUT_MS,
  });

  return {
    municipalityCode: data?.code,
    name: `${data?.pref ?? ""}${data?.ward ?? ""}`,
  };
};
