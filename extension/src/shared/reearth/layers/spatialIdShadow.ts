import { FC, useMemo } from "react";

import { SpatialIdFeature } from "../../spatialId";
import { useLayer } from "../hooks";
import { Data, LayerAppearanceTypes } from "../types";
import { SpatialIdSpaceData } from "../types/reearthPluginAPIv2/spatialId";

export type SpatialIdShadowLayerProps = {
  features: SpatialIdFeature[];
  appearances: Partial<LayerAppearanceTypes>;
  visible?: boolean;
  onLoad?: (layerId: string) => void;
};

export const SpatialIdShadowLayer: FC<SpatialIdShadowLayerProps> = ({
  features,
  appearances,
  visible,
  onLoad,
}) => {
  const data: Data = useMemo(() => {
    const geojsonFeatures = [];
    const uniqueShadowsFeatures = features.reduce((acc, feature) => {
      if (
        !acc.find(
          f =>
            f.data.zfxy.z === feature.data.zfxy.z &&
            f.data.zfxy.x === feature.data.zfxy.x &&
            f.data.zfxy.y === feature.data.zfxy.y,
        )
      ) {
        acc.push(feature);
      }
      return acc;
    }, [] as SpatialIdFeature[]);

    for (const feature of uniqueShadowsFeatures) {
      const { coordinates } = getRectangeParamsFromSpace(feature.data);
      geojsonFeatures.push({
        type: "Feature",
        geometry: {
          coordinates,
          type: "Polygon",
        },
      });
    }
    return {
      type: "geojson",
      value: {
        type: "FeatureCollection",
        features: geojsonFeatures,
      },
    };
  }, [features]);

  useLayer({
    data,
    visible,
    appearances,
    onLoad,
  });

  return null;
};

const getRectangeParamsFromSpace = (space: SpatialIdSpaceData) => {
  const vertices = space.vertices;
  const wsen: [number, number, number, number] = [
    vertices[0][0],
    vertices[1][1],
    vertices[2][0],
    vertices[3][1],
  ];
  const coordinates = [
    [
      [vertices[0][0], vertices[0][1]],
      [vertices[1][0], vertices[1][1]],
      [vertices[2][0], vertices[2][1]],
      [vertices[3][0], vertices[3][1]],
      [vertices[0][0], vertices[0][1]],
    ],
  ];

  const height = vertices[0][2];
  const extrudedHeight = vertices[4][2];
  return { wsen, coordinates, height, extrudedHeight };
};
