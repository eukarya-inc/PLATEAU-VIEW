import { FC, useEffect, useMemo, useRef, useState } from "react";

import { SpatialIdFeature } from "../../spatialId";
import { useLayer, useViewer } from "../hooks";
import { Data, LayerAppearanceTypes } from "../types";
import { SpatialIdSpaceData } from "../types/reearthPluginAPIv2/spatialId";

export type SpatialIdLayerProps = {
  features: SpatialIdFeature[];
  appearances: Partial<LayerAppearanceTypes>;
  visible?: boolean;
  onLoad?: (layerId: string) => void;
};

export const SpatialIdLayer: FC<SpatialIdLayerProps> = ({
  features,
  appearances,
  visible,
  onLoad,
}) => {
  const [geoidHeights, setGeoidHeights] = useState<Record<string, number>>({});
  const geoidHeightsRef = useRef(geoidHeights);
  geoidHeightsRef.current = geoidHeights;

  const { getGeoidHeight } = useViewer();

  useEffect(() => {
    for (const feature of features) {
      const center = feature.data.center;
      if (geoidHeightsRef.current[`${center.lng},${center.lat}`] !== undefined) continue;
      getGeoidHeight(center.lng, center.lat)?.then(geoidHeight => {
        if (geoidHeight === undefined) return;
        setGeoidHeights(prev => ({ ...prev, [`${center.lng},${center.lat}`]: geoidHeight }));
      });
    }
  }, [features, getGeoidHeight]);

  const data: Data = useMemo(() => {
    const geojsonFeatures = [];
    for (const feature of features) {
      const geoidHeight = geoidHeights[`${feature.data.center.lng},${feature.data.center.lat}`];
      if (geoidHeight === undefined) continue;

      const { coordinates, height, extrudedHeight } = getRectangeParamsFromSpace(feature.data);

      geojsonFeatures.push({
        type: "Feature",
        id: feature.id,
        properties: {
          id: feature.id,
          height: height + geoidHeight,
          extrudedHeight: extrudedHeight + geoidHeight,
        },
        geometry: {
          coordinates,
          type: "Polygon",
        },
      });
    }

    return {
      type: "geojson",
      idProperty: "id",
      value: {
        type: "FeatureCollection",
        features: geojsonFeatures,
      },
    };
  }, [features, geoidHeights]);

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
