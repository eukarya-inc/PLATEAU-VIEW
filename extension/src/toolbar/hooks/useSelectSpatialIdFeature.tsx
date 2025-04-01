import { atom, useAtomValue } from "jotai";
import { isEqual } from "lodash-es";
import { useEffect, useMemo, useRef } from "react";

import { parseIdentifier } from "../../prototypes/cesium-helpers";
import { spatialIdLayersAtom } from "../../prototypes/view-layers";
import { spatialIdSelectionAtom } from "../../shared/spatialId";

const selectedSpatialIdLayersAtom = atom(get => {
  const entityIds = get(spatialIdSelectionAtom).map(({ value }) => parseIdentifier(value).key);
  const spatialIdLayers = get(spatialIdLayersAtom);
  const layers: { layerId: string; featureId: string[] }[] = [];
  spatialIdLayers.forEach(layer => {
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

export const useSelectSpatialIdFeature = () => {
  const selectedSpatialIdLayers = useAtomValue(selectedSpatialIdLayersAtom);
  const spatialIdSelection = useAtomValue(spatialIdSelectionAtom);
  const hasSpatialIdFeatureSelected = useMemo(() => {
    return spatialIdSelection.length > 0;
  }, [spatialIdSelection]);

  const prevLayersRef = useRef(selectedSpatialIdLayers);
  useEffect(() => {
    if (!hasSpatialIdFeatureSelected || isEqual(prevLayersRef.current, selectedSpatialIdLayers))
      return;
    setTimeout(() => {
      window.reearth?.layers?.selectFeatures?.(selectedSpatialIdLayers);
      prevLayersRef.current = selectedSpatialIdLayers;
    }, 100);
  }, [hasSpatialIdFeatureSelected, selectedSpatialIdLayers]);
};
