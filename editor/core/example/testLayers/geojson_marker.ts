import { Layer } from "@reearth/core";

export const GEOJSON_MARKER: Layer = {
  id: "geojson_marker",
  type: "simple",
  data: {
    type: "geojson",
    value: {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          properties: {
            index: 1,
          },
          geometry: {
            coordinates: [139.75299772754948, 35.68523972679557],
            type: "Point",
          },
        },
        {
          type: "Feature",
          properties: {
            index: 2,
          },
          geometry: {
            coordinates: [139.8327378666679, 35.67919002820361],
            type: "Point",
          },
        },
        {
          type: "Feature",
          properties: {
            index: 3,
          },
          geometry: {
            coordinates: [139.69716359812975, 35.70030560455889],
            type: "Point",
          },
        },
      ],
    },
  },
  marker: {
    imageColor: {
      expression: {
        conditions: [
          ["${index} === 1", "color('#FF0000')"],
          ["${index} === 2", "color('#00FF00')"],
          ["true", "color('#000000')"],
        ],
      },
    },
  },
};
