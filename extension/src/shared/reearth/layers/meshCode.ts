import { FC, useMemo } from "react";

import { MeshCodeFeature } from "../../meshCode";
import { getCoordinatesFromMeshCode } from "../../meshCode/utils";
import { useLayer } from "../hooks";
import { Data, LayerAppearanceTypes } from "../types";

export type MeshCodeLayerProps = {
  features: MeshCodeFeature[];
  appearances: Partial<LayerAppearanceTypes>;
  visible?: boolean;
  onLoad?: (layerId: string) => void;
};

export const MeshCodeLayer: FC<MeshCodeLayerProps> = ({
  features,
  appearances,
  visible,
  onLoad,
}) => {
  const data: Data = useMemo(() => {
    const geojsonFeatures = [];
    for (const feature of features) {
      const coordinates = getCoordinatesFromMeshCode(feature.meshCode);
      geojsonFeatures.push({
        type: "Feature",
        id: feature.id,
        properties: {
          id: feature.id,
          meshCode: feature.meshCode,
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
  }, [features]);

  useLayer({
    data,
    visible,
    appearances,
    onLoad,
  });

  return null;
};
