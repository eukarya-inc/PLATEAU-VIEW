import { atom, useAtom, useAtomValue, useSetAtom } from "jotai";
import { nanoid } from "nanoid";
import { FC, useCallback, useEffect, useRef } from "react";

import { composeIdentifier } from "../../../prototypes/cesium-helpers";
import { LayerModel, layerSelectionAtom, useAddLayer } from "../../../prototypes/layers";
import { screenSpaceSelectionAtom } from "../../../prototypes/screen-space-selection";
import { spatialIdZoomAtom, toolAtom } from "../../../prototypes/view/states/tool";
import { highlightedSpatialIdLayersAtom, SPATIAL_ID_LAYER } from "../../../prototypes/view-layers";
import { useReEarthEvent } from "../../reearth/hooks";
import { useSpatialId } from "../../reearth/hooks/useSpatialId";
import { SpatialIdSpaceData } from "../../reearth/types/reearthPluginAPIv2/spatialId";
import { SPATIAL_ID_OBJECT, SpatialIdFeature } from "../../spatialId";
import { rootLayersLayersAtom } from "../../states/rootLayer";
import { createRootLayerForLayerAtom } from "../../view-layers";

const targetSpatialIdLayerAtom = atom<LayerModel<typeof SPATIAL_ID_LAYER> | null>(get => {
  const layers = get(rootLayersLayersAtom);
  const selection = get(layerSelectionAtom);
  const selectedSpatialIdLayers = layers.filter(
    (layer): layer is LayerModel<typeof SPATIAL_ID_LAYER> =>
      layer.type === SPATIAL_ID_LAYER && selection.map(s => s.id).includes(layer.id),
  );
  const highlightedSpatialIdLayers = get(highlightedSpatialIdLayersAtom);
  if (selectedSpatialIdLayers.length === 0) {
    return highlightedSpatialIdLayers[0] ?? null;
  }
  return selectedSpatialIdLayers[0] ?? null;
});

const addFeatureAtom = atom(null, (get, set, value: SpatialIdFeature) => {
  const layer = get(targetSpatialIdLayerAtom);
  if (layer == null) {
    return;
  }
  set(layer.featureAtomsAtom, {
    type: "insert",
    value,
  });
});

export const SpatialIdTool: FC = () => {
  const layer = useAtomValue(targetSpatialIdLayerAtom);
  const addFeature = useSetAtom(addFeatureAtom);
  const addLayer = useAddLayer();
  const setScreenSpaceSelection = useSetAtom(screenSpaceSelectionAtom);

  const handleCreate = useCallback(
    (feature: SpatialIdFeature) => {
      const id = nanoid();
      if (layer != null) {
        addFeature(feature);
      } else {
        const layer = createRootLayerForLayerAtom({
          id,
          type: SPATIAL_ID_LAYER,
          features: [feature],
        });
        addLayer(layer, { autoSelect: false });
      }

      setTimeout(() => {
        setScreenSpaceSelection([
          {
            type: SPATIAL_ID_OBJECT,
            value: composeIdentifier({
              type: "SpatialId",
              subtype: SPATIAL_ID_OBJECT,
              key: feature.id,
            }),
          },
        ]);
      }, 320);
    },
    [addFeature, addLayer, setScreenSpaceSelection, layer],
  );

  const [toolType] = useAtom(toolAtom);
  const spatialIdZoom = useAtomValue(spatialIdZoomAtom);
  const { handlePickSpace, handleExitPickSpace } = useSpatialId();

  const zoomRef = useRef(spatialIdZoom);
  zoomRef.current = spatialIdZoom;
  const toolTypeRef = useRef(toolType?.type);
  toolTypeRef.current = toolType?.type;
  const handlePickSpaceRef = useRef(handlePickSpace);
  handlePickSpaceRef.current = handlePickSpace;
  const handleExitPickSpaceRef = useRef(handleExitPickSpace);
  handleExitPickSpaceRef.current = handleExitPickSpace;

  const startPickSpace = useCallback(() => {
    handlePickSpaceRef.current({
      zoom: zoomRef.current,
      rightClickToExit: false,
      dataOnly: true,
      selectorColor: "#00bebe33",
      selectorOutlineColor: "#00bebe00",
      groundIndicatorColor: "#00000033",
    });
  }, []);
  const startPickSpaceRef = useRef(startPickSpace);
  startPickSpaceRef.current = startPickSpace;

  useEffect(() => {
    if (tempSwitchToMoveMode.current) return;
    if (toolType?.type === "spatialId") {
      startPickSpaceRef.current();
    }
  }, [toolType?.type]);

  useEffect(() => {
    if (toolTypeRef.current === "spatialId") {
      handleExitPickSpaceRef.current();
      startPickSpaceRef.current();
    }
  }, [spatialIdZoom]);

  const handleSpatialIdSpacePick = useCallback(
    (data: SpatialIdSpaceData) => {
      if (toolTypeRef.current !== "spatialId") return;
      handleCreate({ id: nanoid(), data });
      startPickSpaceRef.current();
    },
    [handleCreate, startPickSpaceRef, toolTypeRef],
  );

  useReEarthEvent("spatialidspacepick", handleSpatialIdSpacePick);

  const tempSwitchToMoveMode = useRef(false);
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.code === "Space") {
        tempSwitchToMoveMode.current = true;
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  useEffect(() => {
    function handleKeyUp() {
      if (tempSwitchToMoveMode.current === true) {
        tempSwitchToMoveMode.current = false;
      }
    }
    window.addEventListener("keyup", handleKeyUp);
    return () => {
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, []);

  return null;
};
