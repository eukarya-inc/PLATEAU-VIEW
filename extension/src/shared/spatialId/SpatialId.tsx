import { PrimitiveAtom, useAtomValue } from "jotai";
import { FC, useMemo } from "react";

import { SpatialIdShadowLayer } from "../reearth/layers";
import { SpatialIdLayer } from "../reearth/layers/spatialId";
import { LayerAppearanceTypes } from "../reearth/types";

import { SpatialIdObject } from "./SpatialIdObject";
import { SpatialIdFeature } from "./types";

export interface SpatialIdProps {
  featuresAtom: PrimitiveAtom<SpatialIdFeature[]>;
  onLoad: (layerId: string) => void;
}

export const SpatialId: FC<SpatialIdProps> = ({ featuresAtom, onLoad }) => {
  const features = useAtomValue(featuresAtom);

  const appearances: Partial<LayerAppearanceTypes> = useMemo(
    () => ({
      polygon: {
        classificationType: "terrain",
        stroke: false,
        fill: true,
        fillColor: "#00bebe22",
        heightReference: "none",
        hideIndicator: true,
        selectedFeatureColor: "#00bebe55",
        height: {
          expression: "${height}",
        },
        extrudedHeight: {
          expression: "${extrudedHeight}",
        },
      },
    }),
    [],
  );

  const shadowAppearances: Partial<LayerAppearanceTypes> = useMemo(
    () => ({
      polygon: {
        classificationType: "both",
        stroke: false,
        fill: true,
        fillColor: "#00000033",
        heightReference: "none",
        hideIndicator: true,
        clampToGround: true,
      },
    }),
    [],
  );

  return (
    <>
      {features.map((feature, i) => (
        <SpatialIdObject key={i} id={feature.id} />
      ))}
      <SpatialIdLayer features={features} appearances={appearances} onLoad={onLoad} />
      <SpatialIdShadowLayer features={features} appearances={shadowAppearances} />
    </>
  );
};
