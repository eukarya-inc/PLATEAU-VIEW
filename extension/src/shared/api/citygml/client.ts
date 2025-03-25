import { fetchWithGet, fetchWithPost } from "../fetch";

import {
  CityGMLGetAttributesBySpaceData,
  CityGMLGetAttributesBySpaceParams,
  CityGMLGetAttributesData,
  CityGMLGetAttributesParams,
  CityGMLGetFilesData,
  CityGMLGetFilesParams,
  CityGMLGetGmlIdsBySpaceData,
  CityGMLGetGmlIdsBySpaceParams,
  CityGMLGetPackStatusData,
  CityGMLGetPackStatusParams,
  CityGMLPostPackData,
  CityGMLPostPackParams,
} from "./types";

export class CityGMLClient {
  url: string;
  fileUrl: string;

  constructor(baseUrl: string) {
    this.url = `${baseUrl}/citygml`;
    this.fileUrl = `${baseUrl}/datacatalog/citygml`;
  }

  handleError(obj: any) {
    if (!obj) return;
    if (typeof obj !== "object") return;
    if ("error" in obj) {
      console.error(obj.error);
      return;
    }
    return obj;
  }

  async getFiles(params: CityGMLGetFilesParams) {
    const conditions = makeFileAPIConditionsByParams(params);
    if (!conditions) return;
    return this.handleError(
      await fetchWithGet(`${this.fileUrl}/${conditions}`).catch((e: any) => {
        console.error(e);
      }),
    ) as CityGMLGetFilesData;
  }

  async postPack(params: CityGMLPostPackParams) {
    return this.handleError(
      await fetchWithPost(`${this.url}/pack`, params, undefined, "application/json").catch(
        (e: any) => {
          console.error(e);
        },
      ),
    ) as CityGMLPostPackData;
  }

  async getPackStatus({ id }: CityGMLGetPackStatusParams) {
    return this.handleError(
      await fetchWithGet(`${this.url}/pack/${id}/status`).catch((e: any) => {
        console.error(e);
      }),
    ) as CityGMLGetPackStatusData;
  }

  async getAttributes(params: CityGMLGetAttributesParams) {
    const queryParams = new URLSearchParams();
    queryParams.append("url", params.url);
    queryParams.append("id", params.gmlIds.join(","));
    if (params.skipCodeListFetch) queryParams.append("skip_code_list_fetch", "true");
    return this.handleError(
      await fetchWithGet(`${this.url}/attributes?${queryParams}`).catch((e: any) => {
        console.error(e);
      }),
    ) as CityGMLGetAttributesData;
  }

  async getGmlIdsBySpace(params: CityGMLGetGmlIdsBySpaceParams) {
    const queryParams = new URLSearchParams();
    queryParams.append("url", params.url);
    queryParams.append("sid", params.spaceZFXYStrs.join(","));
    return this.handleError(
      await fetchWithGet(`${this.url}/features?${queryParams}`).catch((e: any) => {
        console.error(e);
      }),
    ) as CityGMLGetGmlIdsBySpaceData;
  }

  async getAttributesBySpace(params: CityGMLGetAttributesBySpaceParams) {
    const queryParams = new URLSearchParams();
    queryParams.append("sid", params.spaceZFXYStrs.join(","));
    queryParams.append("type", params.types.join(","));
    return this.handleError(
      await fetchWithGet(`${this.url}/spatialid_attributes?${queryParams}`).catch((e: any) => {
        console.error(e);
      }),
    ) as CityGMLGetAttributesBySpaceData;
  }
}

function makeFileAPIConditionsByParams(params: CityGMLGetFilesParams) {
  const {
    meshIds,
    meshIdsStrict,
    spaceZFXYStrs,
    spaceIds,
    centerCoordinate,
    rangeCoordinates,
    geoName,
    cityIds,
  } = params;
  if (meshIds && meshIds.length > 0) return `m:${meshIds.join(",")}`;
  if (meshIdsStrict && meshIdsStrict.length > 0) return `mm:${meshIdsStrict.join(",")}`;
  if (spaceZFXYStrs && spaceZFXYStrs.length > 0) return `s:${spaceZFXYStrs.join(",")}`;
  if (spaceIds && spaceIds.length > 0) return `s:${spaceIds.join(",")}`;
  if (centerCoordinate) {
    const { lng, lat } = centerCoordinate;
    return `r:${lng},${lat}`;
  }
  if (rangeCoordinates) {
    const { lng1, lat1, lng2, lat2 } = rangeCoordinates;
    return `r:${lng1},${lat1},${lng2},${lat2}`;
  }
  if (geoName) return `g:${geoName}`;
  if (cityIds && cityIds.length > 0) return cityIds.join(",");
  return;
}
