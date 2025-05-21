import type { GeoJSON } from "geojson";

import type { Data, DataRange, Feature } from "../types";

import { f, FetchOptions, generateRandomString } from "./utils";

export async function fetchGeoJSON(
  data: Data,
  range?: DataRange,
  options?: FetchOptions,
): Promise<Feature[] | void> {
  const d = data.url ? await (await f(data.url, options)).json() : data.value;
  return processGeoJSON(d, range, data.idProperty);
}

export function processGeoJSON(
  geojson: GeoJSON,
  range?: DataRange,
  idProperty?: string,
): Feature[] {
  if (geojson.type === "FeatureCollection") {
    return geojson.features.flatMap(f => processGeoJSON(f, range, idProperty));
  }

  if (geojson.type === "Feature") {
    const geo = geojson.geometry;
    if (geo.type === "GeometryCollection") {
      return geo.geometries.flatMap(geometry => {
        return processGeoJSON(
          {
            ...geojson,
            geometry,
          },
          undefined,
          idProperty,
        );
      });
    }
    if (geo.type === "MultiPoint") {
      return geo.coordinates.flatMap(coord => {
        return processGeoJSON(
          {
            ...geojson,
            geometry: {
              type: "Point",
              coordinates: coord,
            },
          },
          undefined,
          idProperty,
        );
      });
    }
    if (geo.type === "MultiLineString") {
      return geo.coordinates.flatMap(coord => {
        return processGeoJSON(
          {
            ...geojson,
            geometry: {
              type: "LineString",
              coordinates: coord,
            },
          },
          undefined,
          idProperty,
        );
      });
    }
    if (geo.type === "MultiPolygon") {
      return geo.coordinates.flatMap(coord => {
        return processGeoJSON(
          {
            ...geojson,
            geometry: {
              type: "Polygon",
              coordinates: coord,
            },
          },
          undefined,
          idProperty,
        );
      });
    }

    return [
      {
        type: "feature",
        id:
          idProperty && geojson.properties && geojson.properties[idProperty]
            ? geojson.properties[idProperty]
            : generateRandomString(12),
        geometry:
          geo.type === "Point" || geo.type === "LineString" || geo.type === "Polygon"
            ? geo
            : undefined,
        properties: geojson.properties,
        range,
      },
    ];
  }

  return [];
}
