import axios from "axios";
import { XMLParser } from "fast-xml-parser";

import { AreasFetcherBase, UPSTREAM_TIMEOUT_MS } from "./types";

const xmlParser = new XMLParser();

export const fetchYahoo: AreasFetcherBase = async (url, lon, lat) => {
  const { data } = await axios.get(url, {
    params: {
      appid: process.env.REVERSE_GEOCODER_API_TOKEN,
      lon,
      lat,
    },
    timeout: UPSTREAM_TIMEOUT_MS,
  });

  const json = xmlParser.parse(data);

  const featureProperty = json["YDF"]?.["Feature"]?.["Property"];
  if (!featureProperty) return;

  const address = featureProperty["Address"];
  const addressElements = featureProperty["AddressElement"];
  const code = (
    addressElements && Array.isArray(addressElements)
      ? addressElements.find(e => e?.["Level"] === "city")
      : undefined
  )?.["Code"];

  return {
    municipalityCode: `${code}`.padStart(5, "0"),
    name: address,
  };
};
