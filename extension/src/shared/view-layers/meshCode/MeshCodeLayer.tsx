import { atom, PrimitiveAtom, useAtomValue, useSetAtom } from "jotai";
import { splitAtom } from "jotai/utils";
import { FC, useCallback } from "react";

import { LayerProps } from "../../../prototypes/layers";
import { SplitAtom } from "../../../prototypes/type-helpers";
import {
  ConfigurableLayerModel,
  createViewLayerModel,
  MESH_CODE_LAYER,
  ViewLayerModel,
  ViewLayerModelParams,
} from "../../../prototypes/view-layers";
import { MeshCode, MeshCodeFeature } from "../../meshCode";

let nextLevel2LayerIndex = 1;
let nextLevel3LayerIndex = 1;

export interface MeshCodeLayerModelParams extends ViewLayerModelParams {
  meshCodeLevel: number;
  features?: readonly MeshCodeFeature[];
}

export interface MeshCodeLayerModel extends ViewLayerModel {
  title: string;
  meshCodeLevel: number;
  featuresAtom: PrimitiveAtom<MeshCodeFeature[]>;
  featureAtomsAtom: SplitAtom<MeshCodeFeature>;
}

export type SharedMeshCodeLayer = {
  type: "meshCode";
  id: string;
  title: string;
  meshCodeLevel: number;
  features: MeshCodeFeature[];
};

export function createMeshCodeLayer(
  params: MeshCodeLayerModelParams,
): ConfigurableLayerModel<MeshCodeLayerModel> {
  const featuresAtom = atom<MeshCodeFeature[]>([...(params.features ?? [])]);
  const title =
    params.meshCodeLevel === 2
      ? `2次 メッシュコレクション ${nextLevel2LayerIndex++}`
      : `3次 メッシュコレクション ${nextLevel3LayerIndex++}`;
  return {
    ...createViewLayerModel({
      ...params,
      title,
    }),
    title,
    type: MESH_CODE_LAYER,
    meshCodeLevel: params.meshCodeLevel,
    featuresAtom,
    featureAtomsAtom: splitAtom(featuresAtom),
  };
}

export const MeshCodeLayer: FC<LayerProps<typeof MESH_CODE_LAYER>> = ({
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
  return <MeshCode featuresAtom={featuresAtom} onLoad={handleLoad} />;
};
