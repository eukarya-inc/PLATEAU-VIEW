import { atom, PrimitiveAtom, useAtomValue, useSetAtom } from "jotai";
import { splitAtom } from "jotai/utils";
import { FC, useCallback } from "react";

import { LayerProps } from "../../../prototypes/layers";
import { SplitAtom } from "../../../prototypes/type-helpers";
import {
  ConfigurableLayerModel,
  createViewLayerModel,
  SPATIAL_ID_LAYER,
  ViewLayerModel,
  ViewLayerModelParams,
} from "../../../prototypes/view-layers";
import { SpatialId, SpatialIdFeature } from "../../spatialId";

let nextLayerIndex = 1;

export interface SpatialIdLayerModelParams extends ViewLayerModelParams {
  features?: readonly SpatialIdFeature[];
}

export interface SpatialIdLayerModel extends ViewLayerModel {
  title: string;
  featuresAtom: PrimitiveAtom<SpatialIdFeature[]>;
  featureAtomsAtom: SplitAtom<SpatialIdFeature>;
}

export type SharedSpatialIdLayer = {
  type: "spatialId";
  id: string;
  title: string;
  features: SpatialIdFeature[];
};

export function createSpatialIdLayer(
  params: SpatialIdLayerModelParams,
): ConfigurableLayerModel<SpatialIdLayerModel> {
  const featuresAtom = atom<SpatialIdFeature[]>([...(params.features ?? [])]);
  const title = `空間IDコレクション${nextLayerIndex++}`;
  return {
    ...createViewLayerModel({
      ...params,
      title,
    }),
    title,
    type: SPATIAL_ID_LAYER,
    featuresAtom,
    featureAtomsAtom: splitAtom(featuresAtom),
  };
}

export const SpatialIdLayer: FC<LayerProps<typeof SPATIAL_ID_LAYER>> = ({
  featuresAtom,
  hiddenAtom,
  layerIdAtom,
}) => {
  const hidden = useAtomValue(hiddenAtom);

  const setLayerId = useSetAtom(layerIdAtom);
  const handleLoad = useCallback(
    (layerId: string) => {
      setLayerId(layerId);
    },
    [setLayerId],
  );

  if (hidden) {
    return null;
  }
  return <SpatialId featuresAtom={featuresAtom} onLoad={handleLoad} />;
};
