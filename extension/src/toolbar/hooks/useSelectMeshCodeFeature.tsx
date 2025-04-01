import { atom, useAtomValue } from "jotai";
import { isEqual } from "lodash-es";
import { useEffect, useMemo, useRef } from "react";

import { parseIdentifier } from "../../prototypes/cesium-helpers";
import { meshCodeLayersAtom } from "../../prototypes/view-layers";
import { meshCodeSelectionAtom } from "../../shared/meshCode";

const selectedMeshCodeLayersAtom = atom(get => {
  const entityIds = get(meshCodeSelectionAtom).map(({ value }) => parseIdentifier(value).key);
  const meshCodeLayers = get(meshCodeLayersAtom);
  const layers: { layerId: string; featureId: string[] }[] = [];
  meshCodeLayers.forEach(layer => {
    const features = get(layer.featuresAtom);
    const layerId = get(layer.layerIdAtom);
    if (!layerId) return;
    const selectedFeatureIds = features
      .filter(f => entityIds.includes(f.id))
      .map(f => String(f.id));
    if (selectedFeatureIds.length) {
      layers.push({ layerId, featureId: selectedFeatureIds });
    }
  });
  return layers;
});

export const useSelectMeshCodeFeature = () => {
  const selectedMeshCodeLayers = useAtomValue(selectedMeshCodeLayersAtom);
  const meshCodeSelection = useAtomValue(meshCodeSelectionAtom);
  const hasMeshCodeFeatureSelected = useMemo(() => {
    return meshCodeSelection.length > 0;
  }, [meshCodeSelection]);

  const prevLayersRef = useRef(selectedMeshCodeLayers);
  useEffect(() => {
    if (!hasMeshCodeFeatureSelected || isEqual(prevLayersRef.current, selectedMeshCodeLayers))
      return;
    setTimeout(() => {
      window.reearth?.layers?.selectFeatures?.(selectedMeshCodeLayers);
      prevLayersRef.current = selectedMeshCodeLayers;
    }, 100);
  }, [hasMeshCodeFeatureSelected, selectedMeshCodeLayers]);
};
