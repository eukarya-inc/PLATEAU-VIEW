import { memo, forwardRef, CSSProperties, type Ref, type PropsWithChildren } from "react";
import { ErrorBoundary } from "react-error-boundary";

import { ComputedFeature } from "../mantle";
import {
  Map,
  type MapRef,
  type ViewerProperty,
  type Layer,
  type LayerSelectionReason,
  type Camera,
  type LatLng,
  type Cluster,
  type ComputedLayer,
  type Credit,
} from "../Map";
import { SketchFeature, SketchType } from "../Map/Sketch/types";

import { VisualizerProvider } from "./context";
import { coreContext } from "./coreContext";
import { engines, type EngineType } from "./engines";
import Err from "./Error";
import useHooks from "./hooks";
import type { InteractionModeType } from "./interactionMode";

export { useVisualizer, type Context as VisualizerContext } from "./context";
export * from "./engines";
export * from "./useViewport";
export * from "./coreContext";
export * from "./featureFlags";
export * from "./interactionMode";

export type CoreVisualizerProps = {
  engine?: EngineType;
  isBuilt?: boolean;
  isEditable?: boolean;
  viewerProperty?: ViewerProperty;
  layers?: Layer[];
  clusters?: Cluster[]; // TODO: remove completely from beta core
  time?: string | Date;
  camera?: Camera;
  interactionMode?: InteractionModeType;
  shouldRender?: boolean;
  meta?: Record<string, unknown>;
  style?: CSSProperties;
  small?: boolean;
  ready?: boolean;
  hiddenLayers?: string[];
  zoomedLayerId?: string;
  displayCredits?: boolean;
  onCameraChange?: (camera: Camera) => void;
  onLayerDrop?: (layerId: string, propertyKey: string, position: LatLng | undefined) => void;
  onLayerSelect?: (
    layerId: string | undefined,
    layer: (() => Promise<ComputedLayer | undefined>) | undefined,
    feature: ComputedFeature | undefined,
    reason: LayerSelectionReason | undefined,
  ) => void;
  onZoomToLayer?: (layerId: string | undefined) => void;
  onMount?: () => void;
  onSketchTypeChangeProp?: (type: SketchType | undefined) => void;
  onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
  onSketchFeatureUpdate?: (feature: SketchFeature | null) => void;
  onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
  onInteractionModeChange?: (mode: InteractionModeType) => void;
  onAPIReady?: () => void;
  onCreditsUpdate?: (credits: Credit[]) => void;
};

export const CoreVisualizer = memo(
  forwardRef<MapRef, PropsWithChildren<CoreVisualizerProps>>(
    (
      {
        engine,
        isBuilt,
        isEditable,
        viewerProperty,
        layers,
        clusters,
        small,
        ready,
        hiddenLayers,
        camera: initialCamera,
        time,
        interactionMode,
        shouldRender,
        meta,
        displayCredits = true,
        style,
        zoomedLayerId,
        children,
        onLayerDrop,
        onLayerSelect,
        onCameraChange,
        onZoomToLayer,
        onInteractionModeChange,
        onMount,
        onSketchTypeChangeProp,
        onSketchFeatureCreate,
        onSketchFeatureUpdate,
        onSketchFeatureDelete,
        onAPIReady,
        onCreditsUpdate,
      },
      ref: Ref<MapRef | null>,
    ) => {
      const {
        mapRef,
        wrapperRef,
        selectedFeature,
        camera,
        featureFlags,
        isLayerDragging,
        timelineManagerRef,
        cameraForceHorizontalRoll,
        coreContextValue,
        containerStyle,
        overriddenInteractionMode,
        handleLayerSelect,
        handleLayerDrag,
        handleLayerDrop,
        handleLayerEdit,
        handleCameraChange,
        handleInteractionModeChange,
        handleSketchPluginFeatureCreate,
        handleSketchPluginFeatureUpdate,
        handleSketchPluginFeatureDelete,
        handleSketchTypeChange,
        handleLayerVisibility,
        handleLayerLoad,
        handleLayerSelectWithRectStart,
        handleLayerSelectWithRectMove,
        handleLayerSelectWithRectEnd,
      } = useHooks(
        {
          camera: initialCamera,
          interactionMode,
          zoomedLayerId,
          viewerProperty,
          onLayerSelect,
          onCameraChange,
          onZoomToLayer,
          onLayerDrop,
          onInteractionModeChange,
          onSketchTypeChangeProp,
        },
        ref,
      );

      return (
        <ErrorBoundary FallbackComponent={Err}>
          <VisualizerProvider mapRef={mapRef}>
            <div ref={wrapperRef} style={containerStyle}>
              <Map
                ref={mapRef}
                isBuilt={isBuilt}
                isEditable={isEditable}
                engine={engine}
                layers={layers}
                engines={engines}
                camera={camera}
                cameraForceHorizontalRoll={cameraForceHorizontalRoll}
                clusters={clusters}
                hiddenLayers={hiddenLayers}
                isLayerDragging={isLayerDragging}
                isLayerDraggable={isEditable}
                meta={meta}
                displayCredits={displayCredits}
                style={style}
                featureFlags={featureFlags}
                shouldRender={shouldRender}
                property={viewerProperty}
                time={time}
                small={small}
                ready={ready}
                timelineManagerRef={timelineManagerRef}
                interactionMode={overriddenInteractionMode}
                selectedFeature={selectedFeature}
                onCameraChange={handleCameraChange}
                onLayerDrag={handleLayerDrag}
                onLayerDrop={handleLayerDrop}
                onLayerSelect={handleLayerSelect}
                onLayerEdit={handleLayerEdit}
                overrideInteractionMode={handleInteractionModeChange}
                onSketchFeatureCreate={onSketchFeatureCreate}
                onSketchPluginFeatureCreate={handleSketchPluginFeatureCreate}
                onSketchFeatureUpdate={onSketchFeatureUpdate}
                onSketchPluginFeatureUpdate={handleSketchPluginFeatureUpdate}
                onSketchFeatureDelete={onSketchFeatureDelete}
                onSketchPluginFeatureDelete={handleSketchPluginFeatureDelete}
                onSketchTypeChange={handleSketchTypeChange}
                onMount={onMount}
                onLayerVisibility={handleLayerVisibility}
                onLayerLoad={handleLayerLoad}
                onLayerSelectWithRectStart={handleLayerSelectWithRectStart}
                onLayerSelectWithRectMove={handleLayerSelectWithRectMove}
                onLayerSelectWithRectEnd={handleLayerSelectWithRectEnd}
                onAPIReady={onAPIReady}
                onCreditsUpdate={onCreditsUpdate}
              />
              <coreContext.Provider value={coreContextValue}>{children}</coreContext.Provider>
            </div>
          </VisualizerProvider>
        </ErrorBoundary>
      );
    },
  ),
);
