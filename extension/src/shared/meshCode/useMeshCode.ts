import jismesh from "jismesh-js";
import { atom, useAtomValue, useSetAtom } from "jotai";
import { nanoid } from "nanoid";
import { useCallback } from "react";

import { composeIdentifier } from "../../prototypes/cesium-helpers";
import { LayerModel, layerSelectionAtom, useAddLayer } from "../../prototypes/layers";
import { screenSpaceSelectionAtom } from "../../prototypes/screen-space-selection";
import { MESH_CODE_LAYER, highlightedMeshCodeLayersAtom } from "../../prototypes/view-layers";
import { rootLayersLayersAtom } from "../states/rootLayer";
import { createRootLayerForLayerAtom } from "../view-layers";

import { MESH_CODE_LAYER_MAX_OBJECTS } from "./constants";
import { MESH_CODE_OBJECT, MeshCodeFeature, MeshCodeType } from "./types";
import { getMeshCodeLevelByType } from "./utils";

const targetMeshCodeLayerAtom = atom<LayerModel<typeof MESH_CODE_LAYER> | null>(get => {
  const layers = get(rootLayersLayersAtom);
  const selection = get(layerSelectionAtom);
  const selectedMeshCodeLayers = layers.filter(
    (layer): layer is LayerModel<typeof MESH_CODE_LAYER> =>
      layer.type === MESH_CODE_LAYER && selection.map(s => s.id).includes(layer.id),
  );
  const highlightedMeshCodeLayers = get(highlightedMeshCodeLayersAtom);
  if (selectedMeshCodeLayers.length === 0) {
    return highlightedMeshCodeLayers[0] ?? null;
  }
  return selectedMeshCodeLayers[0] ?? null;
});

const existMeshCodeFeaturesAtom = atom(get => {
  const layer = get(targetMeshCodeLayerAtom);
  return layer?.featuresAtom ? get(layer.featuresAtom) : [];
});

const addFeatureAtom = atom(null, (get, set, value: MeshCodeFeature) => {
  const layer = get(targetMeshCodeLayerAtom);
  if (layer == null) {
    return;
  }
  set(layer.featureAtomsAtom, {
    type: "insert",
    value,
  });
});

const removeFeatureByIdAtom = atom(null, (get, set, id: string) => {
  const layer = get(targetMeshCodeLayerAtom);
  if (layer == null) {
    return;
  }
  const featureAtom = get(layer.featureAtomsAtom).find(f => get(f).id === id);
  if (!featureAtom) return;
  set(layer.featureAtomsAtom, {
    type: "remove",
    atom: featureAtom,
  });
});

export default ({ meshCodeType }: { meshCodeType: MeshCodeType }) => {
  const layer = useAtomValue(targetMeshCodeLayerAtom);
  const addFeature = useSetAtom(addFeatureAtom);
  const removeFeatureById = useSetAtom(removeFeatureByIdAtom);
  const addLayer = useAddLayer();
  const setScreenSpaceSelection = useSetAtom(screenSpaceSelectionAtom);

  const existFeatures = useAtomValue(existMeshCodeFeaturesAtom);

  const handleCreate = useCallback(
    ({
      meshCode: meshCodeInput,
      location,
      forceCreateNewLayer,
    }: {
      meshCode?: string;
      location?: { lat: number; lng: number };
      forceCreateNewLayer?: boolean;
    }) => {
      let meshCode = meshCodeInput;
      if (!meshCode && location) {
        meshCode = jismesh.toMeshCode(
          location.lat,
          location.lng,
          getMeshCodeLevelByType(meshCodeType),
        );
      }

      if (!meshCode) return;

      if (existFeatures?.some(f => f.meshCode === meshCode)) {
        return;
      }

      const id = nanoid();
      const feature: MeshCodeFeature = {
        id,
        meshCode,
      };

      const meshCodeLevel = getMeshCodeLevelByType(meshCodeType);

      if (layer != null && layer.meshCodeLevel === meshCodeLevel && !forceCreateNewLayer) {
        // limit the maximum number of features
        // if the number of features exceeds the limit, remove the oldest feature
        if (existFeatures.length >= MESH_CODE_LAYER_MAX_OBJECTS) {
          removeFeatureById(existFeatures[0].id);
        }
        addFeature(feature);
      } else {
        const layer = createRootLayerForLayerAtom({
          id,
          type: MESH_CODE_LAYER,
          meshCodeLevel,
          features: [feature],
        });
        addLayer(layer, { autoSelect: false });
      }

      setScreenSpaceSelection([
        {
          type: MESH_CODE_OBJECT,
          value: composeIdentifier({
            type: "MeshCode",
            subtype: MESH_CODE_OBJECT,
            key: feature.id,
          }),
        },
      ]);
    },
    [
      addFeature,
      removeFeatureById,
      addLayer,
      setScreenSpaceSelection,
      layer,
      existFeatures,
      meshCodeType,
    ],
  );

  return {
    handleCreate,
  };
};
