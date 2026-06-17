import axios from "axios";

import { AreasFetcherBase, UPSTREAM_TIMEOUT_MS } from "./types";

export const fetchGSI: AreasFetcherBase = async (url, lon, lat) => {
  const { data } = await axios.get(url, {
    params: {
      lon,
      lat,
    },
    timeout: UPSTREAM_TIMEOUT_MS,
  });
  const municipalityCode = data.results?.muniCd;
  const name = data.results?.lv01Nm;

  return {
    municipalityCode,
    name,
  };
};
