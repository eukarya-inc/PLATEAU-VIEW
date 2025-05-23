import { forwardRef, RefObject, type ForwardRefRenderFunction, type MutableRefObject } from "react";

import { SelectedFeatureInfo } from "../../mantle";
import ClusteredLayers, { type Props as ClusteredLayerProps } from "../ClusteredLayers";
import type { ComputedLayer, EngineRef, RequestingRenderMode, ViewerProperty } from "../types";

import useHooks, { LayerSelectionReason, type Ref } from "./hooks";

export type {
  CommonProps,
  FeatureComponentProps,
  FeatureComponentType,
  Layer,
  LayerSimple,
  EvalFeature,
} from "../Layer";
export type {
  LazyLayer,
  Ref,
  NaiveLayer,
  LayerSelectionReason,
  DefaultInfobox,
  OverriddenLayer,
} from "./hooks";
export type {
  ClusterComponentType,
  ClusterComponentProps,
  ClusterProperty,
  Cluster,
} from "../ClusteredLayers";

export type Props = Omit<ClusteredLayerProps, "atomMap" | "isHidden" | "selectedLayerId"> & {
  selectedLayer?: {
    layerId?: string;
    featureId?: string;
    reason?: LayerSelectionReason;
  };
  hiddenLayers?: string[];
  viewerProperty?: ViewerProperty;
  requestingRenderMode?: MutableRefObject<RequestingRenderMode>;
  engineRef?: RefObject<EngineRef>;
  onLayerSelect?: (
    layerId: string | undefined,
    featureId: string | undefined,
    layer: (() => Promise<ComputedLayer | undefined>) | undefined,
    reason: LayerSelectionReason | undefined,
    info: SelectedFeatureInfo | undefined,
  ) => void;
  onMount?: () => void;
};

const Layers: ForwardRefRenderFunction<Ref, Props> = (
  {
    layers,
    selectedLayer,
    hiddenLayers,
    requestingRenderMode,
    engineRef,
    onLayerSelect,
    onMount,
    ...props
  },
  ref,
) => {
  const { atomMap, flattenedLayers, isHidden } = useHooks({
    layers,
    ref,
    hiddenLayers,
    selectedLayer,
    requestingRenderMode,
    engineRef,
    onLayerSelect,
    onMount,
  });

  return (
    <ClusteredLayers
      {...props}
      selectedLayer={selectedLayer}
      layers={flattenedLayers}
      atomMap={atomMap}
      isHidden={isHidden}
    />
  );
};

export default forwardRef(Layers);
