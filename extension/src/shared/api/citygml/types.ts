export type CityGMLGetFilesParams = {
  meshIds?: string[];
  meshIdsStrict?: string[];
  spaceZFXYStrs?: string[];
  spaceIds?: string[];
  centerCoordinate?: {
    lng: number;
    lat: number;
  };
  rangeCoordinates?: {
    lng1: number;
    lat1: number;
    lng2: number;
    lat2: number;
  };
  geoName?: string;
  cityIds?: string[];
};

export type CityGMLFileCityData = {
  cityCode: string;
  cityName: string;
  year: number;
  registrationYear: number;
  spec: string;
  url: string;
  metadataZipUrls: string[];
  files: {
    [key: string]: {
      code: string;
      maxLod: number;
      url: string;
    }[];
  };
};

export type CityGMLGetFilesData = {
  cities: CityGMLFileCityData[];
  featureTypes: {
    [key: string]: {
      name: string;
    };
  };
};

export type CityGMLPostPackParams = {
  urls: string[];
};

export type CityGMLPostPackData = {
  id: string;
};

export type CityGMLGetPackStatusParams = {
  id: string;
};

export type CityGMLGetPackStatusData = {
  status: "accepted" | "processing" | "succeeded" | "failed";
  startedAt?: string;
  progress?: number;
};

export type CityGMLGetAttributesParams = {
  url: string;
  gmlIds: string[];
  skipCodeListFetch?: boolean;
};

export type CityGMLGetAttributesData = Record<string, any>[];

export type CityGMLGetGmlIdsBySpaceParams = {
  url: string;
  spaceZFXYStrs: string[];
};

export type CityGMLGetGmlIdsBySpaceData = {
  featureIds: string[];
};

export type CityGMLGetAttributesBySpaceParams = {
  spaceZFXYStrs: string[];
  types: string[];
};

export type CityGMLGetAttributesBySpaceData = Record<string, any>[];

export type CityGMLGetPackSizeParams = {
  id: string;
};

export type CityGMLPackItem = {
  id: string;
  name: string;
  fileUrls: string[];
  status: "idle" | "requesting" | "polling" | "packed" | "retry";
  progress: number;
  pollingCount: number;
  packId?: string;
  packSize?: number;
};
