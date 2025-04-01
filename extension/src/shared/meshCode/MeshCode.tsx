import { PrimitiveAtom, useAtomValue } from "jotai";
import { FC, useMemo } from "react";

import { MeshCodeLayer } from "../reearth/layers";
import { LayerAppearanceTypes } from "../reearth/types";

import { MeshCodeObject } from "./MeshCodeObject";
import { MeshCodeFeature } from "./types";

export interface MeshCodeProps {
  featuresAtom: PrimitiveAtom<MeshCodeFeature[]>;
  onLoad: (layerId: string) => void;
}

export const MeshCode: FC<MeshCodeProps> = ({ featuresAtom, onLoad }) => {
  const features = useAtomValue(featuresAtom);

  const appearances: Partial<LayerAppearanceTypes> = useMemo(
    () => ({
      polygon: {
        fill: true,
        fillColor: "#00bebe22",
        stroke: true,
        strokeColor: "#00bebe",
        heightReference: "clamp" as const,
        hideIndicator: true,
        selectedFeatureColor: "#00bebe99",
      },
    }),
    [],
  );

  return (
    <>
      {features.map((feature, i) => (
        <MeshCodeObject key={i} id={feature.id} />
      ))}
      <MeshCodeLayer features={features} appearances={appearances} onLoad={onLoad} />
    </>
  );
};
