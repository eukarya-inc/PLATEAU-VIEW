import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  ComputedFeature,
  ComputedLayer,
  Credit,
  LayerSelectionReason,
  LazyLayer,
  MapRef,
  SketchEditingFeature,
  SketchType,
  ViewerProperty,
} from "@reearth/core";

import { DEFAULT_CAMERA, DEFAULT_LAYERS, DEFAULT_TILE } from "./constants";
import { DEFAULT_VIEWER_PROPERTY } from "./scene";
import { TEST_LAYERS } from "./testLayers";

export default () => {
  const ref = useRef<MapRef>(null);
  const [isReady, setIsReady] = useState(false);
  const handleMount = useCallback(() => {
    requestAnimationFrame(() => {
      setIsReady(true);
    });
  }, []);

  const handleAPIReady = useCallback(() => {
    ref.current?.sketch.overrideOptions({
      dataOnly: false,
    });
    ref.current?.sketch.onEditFeatureChange(setSketchEditingFeatrue);
  }, []);

  // TODO: use onLayerSelect props (core should export a type for selection).
  const [selectedLayer, setSelectedLayer] = useState<LazyLayer | undefined>();
  const [selectedFeature, setSelectedFeature] = useState<ComputedFeature | undefined>();
  const handleSelect: (
    layerId: string | undefined,
    layer: (() => Promise<ComputedLayer | undefined>) | undefined,
    feature: ComputedFeature | undefined,
    reason: LayerSelectionReason | undefined,
  ) => void = useCallback((_layerId, _layer, feature) => {
    setSelectedLayer(ref.current?.layers.selectedLayer());
    // console.log("SELECTED: ", feature?.properties);
    setSelectedFeature(ref.current?.layers.selectedFeature() ?? feature);
  }, []);

  const meta = useMemo(
    () => ({
      cesiumIonAccessToken: import.meta.env.EXAMPLE_CESIUM_ION_ACCESS_TOKEN || undefined,
    }),
    [],
  );

  const [currentTile, setCurrentTile] = useState(DEFAULT_TILE);
  const [currentCamera, setCurrentCamera] = useState(DEFAULT_CAMERA);
  const [terrainEnabled, setTerrainEnabled] = useState(true);
  const [hideUnderground, setHideUnderground] = useState(false);
  const [activeLayerIds, setActiveLayerIds] = useState<string[]>(DEFAULT_LAYERS);

  const viewerProperty: ViewerProperty = useMemo(
    () => ({
      ...DEFAULT_VIEWER_PROPERTY,
      tiles: [
        {
          id: "default",
          type: currentTile,
          opacity: 1,
        },
      ],
      terrain: {
        ...DEFAULT_VIEWER_PROPERTY.terrain,
        enabled: terrainEnabled,
      },
      globe: {
        ...DEFAULT_VIEWER_PROPERTY.globe,
        depthTestAgainstTerrain: hideUnderground,
      },
    }),
    [currentTile, terrainEnabled, hideUnderground],
  );

  const layers = useMemo(
    () => TEST_LAYERS.filter(layer => activeLayerIds.includes(layer.id)),
    [activeLayerIds],
  );

  const [sketchTool, setSketchTool] = useState<SketchType | undefined>(undefined);
  useEffect(() => {
    ref.current?.sketch?.setType(sketchTool);
  }, [ref, sketchTool]);

  const sketchFeatureSelected = useMemo(
    () =>
      !!(
        !!selectedFeature &&
        selectedLayer?.type === "simple" &&
        selectedLayer.data?.isSketchLayer
      ),
    [selectedLayer, selectedFeature],
  );

  const [sketchEditingFeature, setSketchEditingFeatrue] = useState<
    SketchEditingFeature | undefined
  >();

  const handleEditSketchFeature = useCallback(() => {
    if (
      !(selectedLayer?.type === "simple" && selectedLayer.data?.isSketchLayer) ||
      !selectedLayer.id ||
      !selectedFeature?.id
    )
      return;
    ref.current?.sketch.editFeature({ layerId: selectedLayer.id, feature: selectedFeature });
  }, [selectedLayer, selectedFeature]);

  const handleCancelEditSketchFeature = useCallback(() => {
    ref.current?.sketch.cancelEdit();
  }, []);

  const handleApplyEditSketchFeature = useCallback(() => {
    ref.current?.sketch.applyEdit();
  }, []);

  const handleDeleteSketchFeature = useCallback(() => {
    if (
      !(selectedLayer?.type === "simple" && selectedLayer.data?.isSketchLayer) ||
      !selectedLayer.id ||
      !selectedFeature?.id
    )
      return;
    ref.current?.sketch.deleteFeature(selectedLayer.id, selectedFeature.id);
  }, [selectedLayer, selectedFeature]);

  const [_credits, setCredits] = useState<Credit[]>([]);
  const handleCreditsUpdate = useCallback((credits: Credit[]) => {
    setCredits(credits);
  }, []);

  const handleGetCredits = useCallback(() => {
    alert(JSON.stringify(ref.current?.engine?.getCredits()));
  }, []);

  // Spatial ID
  const [spatialIdZoom, setSpatialIdZoom] = useState<number>(20);
  const handleSpatialIdZoomChange = useCallback((value: number[]) => {
    setSpatialIdZoom(value[0]);
  }, []);

  const handleSpatialIdPick = useCallback(() => {
    ref.current?.spatialId?.pickSpace({ zoom: spatialIdZoom });
  }, [spatialIdZoom]);

  return {
    isReady,
    ref,
    handleMount,
    handleAPIReady,
    handleSelect,
    meta,
    currentTile,
    setCurrentTile,
    currentCamera,
    setCurrentCamera,
    terrainEnabled,
    setTerrainEnabled,
    hideUnderground,
    setHideUnderground,
    activeLayerIds,
    setActiveLayerIds,
    viewerProperty,
    layers,
    sketchTool,
    setSketchTool,
    selectedLayer,
    selectedFeature,
    sketchEditingFeature,
    sketchFeatureSelected,
    handleEditSketchFeature,
    handleCancelEditSketchFeature,
    handleApplyEditSketchFeature,
    handleDeleteSketchFeature,
    handleCreditsUpdate,
    handleGetCredits,
    handleSpatialIdPick,
    spatialIdZoom,
    handleSpatialIdZoomChange,
  };
};
