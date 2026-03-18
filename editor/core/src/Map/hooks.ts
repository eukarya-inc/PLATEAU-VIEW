import { useImperativeHandle, useRef, type Ref, useState, useCallback, useEffect } from "react";

import { SelectedFeatureInfo } from "../mantle";

import { GeoidRef } from "./Geoid/types";
import { type MapRef, mapRef } from "./ref";
import { SpatialIdRef } from "./SpatialId/types";
import type {
  EngineRef,
  LayersRef,
  LayerSelectionReason,
  ComputedLayer,
  RequestingRenderMode,
  SketchRef,
} from "./types";
import useTimelineManager, { TimelineManagerRef } from "./useTimelineManager";

import { SketchEditingFeature } from ".";

export type { MapRef } from "./ref";

export const FORCE_REQUEST_RENDER = -1;
export const NO_REQUEST_RENDER = 0;
export const REQUEST_RENDER_ONCE = 1;

export default function ({
  ref,
  timelineManagerRef,
  onLayerSelect,
  onMount,
  onAPIReady,
}: {
  ref: Ref<MapRef>;
  timelineManagerRef?: TimelineManagerRef;
  onLayerSelect?: (
    layerId: string | undefined,
    featureId: string | undefined,
    layer: (() => Promise<ComputedLayer | undefined>) | undefined,
    options?: LayerSelectionReason,
    info?: SelectedFeatureInfo,
  ) => void;
  onMount?: () => void;
  onAPIReady?: () => void;
}) {
  const [mapAPIReady, setMapAPIReady] = useState({
    engine: false,
    layers: false,
    sketch: false,
    spatialId: false,
  });
  const engineRef = useRef<EngineRef>(null);
  const layersRef = useRef<LayersRef>(null);
  const sketchRef = useRef<SketchRef>(null);
  const spatialIdRef = useRef<SpatialIdRef>(null);
  const geoidRef = useRef<GeoidRef>(null);
  const requestingRenderMode = useRef<RequestingRenderMode>(NO_REQUEST_RENDER);

  useImperativeHandle(
    ref,
    () =>
      mapRef({
        engineRef,
        layersRef,
        sketchRef,
        spatialIdRef,
        geoidRef,
        timelineManagerRef,
      }),
    [timelineManagerRef],
  );

  useEffect(() => {
    if (
      onAPIReady &&
      mapAPIReady.engine &&
      mapAPIReady.layers &&
      mapAPIReady.sketch &&
      mapAPIReady.spatialId
    ) {
      onAPIReady?.();
    }
  }, [onAPIReady, mapAPIReady]);

  // selectLayer logic
  // 1. Map/hooks(here) is the source
  //    1.2 State updates propagate up, through onLayerSelect, to update
  //        the pluginAPI(in Crust) and to update external state through
  //        the Visualizer's onLayerselect prop.
  // 2. Passes down from Map to Layers
  // 3. Passes down from Map to Engine
  // 4. Source state can be updated only from the Engine (through the layersRef)

  const [selectedLayer, selectLayer] = useState<{
    layerId?: string;
    featureId?: string;
    reason?: LayerSelectionReason;
  }>({});

  const handleLayerSelect = useCallback(
    async (
      layerId: string | undefined,
      featureId: string | undefined,
      layer: (() => Promise<ComputedLayer | undefined>) | undefined,
      reason?: LayerSelectionReason,
      info?: SelectedFeatureInfo,
    ) => {
      selectLayer({ layerId, featureId, reason });
      onLayerSelect?.(layerId, featureId, layer, reason, info);
    },
    [onLayerSelect],
  );

  const handleEngineLayerSelect = useCallback(
    (
      layerId: string | undefined,
      featureId?: string,
      reason?: LayerSelectionReason,
      info?: SelectedFeatureInfo,
    ) => {
      layersRef.current?.selectFeatures(
        [{ layerId, featureId: featureId ? [featureId] : undefined }],
        reason,
        info,
      );
    },
    [],
  );

  useTimelineManager({
    engineRef,
    timelineManagerRef,
  });

  const [sketchEditingFeature, setSketchEditingFeature] = useState<
    SketchEditingFeature | undefined
  >();

  const handleEngineMount = useCallback(() => {
    setMapAPIReady(s => ({ ...s, engine: true }));
    onMount?.();
  }, [onMount]);
  const handleLayersMount = useCallback(() => {
    setMapAPIReady(s => ({ ...s, layers: true }));
  }, []);
  const handleSketchMount = useCallback(() => {
    setMapAPIReady(s => ({ ...s, sketch: true }));
  }, []);
  const handleSpatialIdMount = useCallback(() => {
    setMapAPIReady(s => ({ ...s, spatialId: true }));
  }, []);

  return {
    engineRef,
    layersRef,
    sketchRef,
    spatialIdRef,
    geoidRef,
    selectedLayer,
    requestingRenderMode,
    handleLayerSelect,
    handleEngineLayerSelect,
    sketchEditingFeature,
    setSketchEditingFeature,
    handleEngineMount,
    handleLayersMount,
    handleSketchMount,
    handleSpatialIdMount,
  };
}
