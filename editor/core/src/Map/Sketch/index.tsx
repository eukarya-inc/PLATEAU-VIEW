// Reference: Sketch feature is basically referenced from https://github.com/takram-design-engineering/plateau-view/blob/main/libs/sketch/src/SketchTool.tsx

import { ForwardRefRenderFunction, RefObject, forwardRef } from "react";

import { ComputedLayer, SelectedFeatureInfo } from "../../mantle";
import { InteractionModeType } from "../../Visualizer/interactionMode";
import { EngineRef, Feature, LayerSelectionReason, LayersRef, SketchRef } from "../types";

import useHooks from "./hooks";
import {
  SketchComponentType,
  SketchEditingFeature,
  SketchEventProps,
  SketchFeature,
  SketchType,
} from "./types";

export * from "./types";

export type OnLayerSelectType = (
  layerId: string | undefined,
  featureId: string | undefined,
  layer: (() => Promise<ComputedLayer | undefined>) | undefined,
  reason: LayerSelectionReason | undefined,
  info: SelectedFeatureInfo | undefined,
) => void;

export type SketchProps = {
  layersRef: RefObject<LayersRef>;
  engineRef: RefObject<EngineRef>;
  SketchComponent?: SketchComponentType;
  selectedFeature?: Feature;
  interactionMode?: InteractionModeType;
  overrideInteractionMode?: (mode: InteractionModeType) => void;
  onSketchTypeChange?: (type: SketchType | undefined, from?: "editor" | "plugin") => void;
  onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
  onSketchPluginFeatureCreate?: (props: SketchEventProps) => void;
  onSketchFeatureUpdate?: (feature: SketchFeature | null) => void;
  onSketchPluginFeatureUpdate?: (props: SketchEventProps) => void;
  onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
  onSketchPluginFeatureDelete?: (props: { layerId: string; featureId: string }) => void;
  onLayerSelect?: OnLayerSelectType;
  sketchEditingFeature?: SketchEditingFeature;
  onSketchEditFeature?: (feature: SketchEditingFeature | undefined) => void;
  onMount?: () => void;
};

const Sketch: ForwardRefRenderFunction<SketchRef, SketchProps> = (
  {
    layersRef,
    engineRef,
    interactionMode = "default",
    selectedFeature,
    SketchComponent,
    overrideInteractionMode,
    onSketchTypeChange,
    onSketchFeatureCreate,
    onSketchPluginFeatureCreate,
    onSketchFeatureUpdate,
    onSketchPluginFeatureUpdate,
    onSketchFeatureDelete,
    onSketchPluginFeatureDelete,
    onLayerSelect,
    sketchEditingFeature,
    onSketchEditFeature,
    onMount,
  },
  ref,
) => {
  const {
    state,
    isEditing,
    catchedControlPointIndex,
    catchedExtrudedPoint,
    extrudedHeight,
    extrudedPoint,
    centroidBasePoint,
    centroidExtrudedPoint,
    geometryOptions,
    color,
    disableShadow,
    handleControlPointMouseEvent,
    handleAddControlPoint,
    selectedControlPointIndex,
  } = useHooks({
    ref,
    layersRef,
    engineRef,
    interactionMode,
    selectedFeature,
    overrideInteractionMode,
    onSketchTypeChange,
    onSketchFeatureCreate,
    onSketchPluginFeatureCreate,
    onSketchFeatureUpdate,
    onSketchPluginFeatureUpdate,
    onSketchFeatureDelete,
    onSketchPluginFeatureDelete,
    onLayerSelect,
    sketchEditingFeature,
    onSketchEditFeature,
    onMount,
  });
  if (state.matches("idle")) {
    return null;
  }
  return SketchComponent ? (
    <SketchComponent
      geometryOptions={geometryOptions}
      color={color}
      disableShadow={disableShadow}
      isEditing={isEditing}
      catchedControlPointIndex={catchedControlPointIndex}
      catchedExtrudedPoint={catchedExtrudedPoint}
      extrudedHeight={extrudedHeight}
      extrudedPoint={extrudedPoint}
      centroidBasePoint={centroidBasePoint}
      centroidExtrudedPoint={centroidExtrudedPoint}
      handleControlPointMouseEvent={handleControlPointMouseEvent}
      handleAddControlPoint={handleAddControlPoint}
      selectedControlPointIndex={selectedControlPointIndex}
    />
  ) : null;
};

export default forwardRef(Sketch);
