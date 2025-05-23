import { Dispatch, RefObject, SetStateAction, useCallback } from "react";

import { LazyLayer } from "../Layers";
import { LayersRef } from "../types";

import { SketchEventProps, SketchFeature, SketchOptions, SketchType } from "./types";

import { OnLayerSelectType } from ".";

type Props = {
  sketchOptions: SketchOptions;
  from: "editor" | "plugin";
  updateType: Dispatch<SetStateAction<SketchType | undefined>>;
  layersRef: RefObject<LayersRef>;
  pluginSketchLayerCreate: (feature: SketchFeature) => {
    layerId: string | undefined;
    featureId: string;
  };
  pluginSketchLayerFeatureAdd: (
    layer: LazyLayer,
    feature: SketchFeature,
  ) =>
    | {
        layerId?: undefined;
        featureId?: undefined;
      }
    | {
        layerId: string;
        featureId: string;
      };
  pluginSketchLayerFeatureUpdate: (
    layer: LazyLayer,
    feature: SketchFeature,
  ) =>
    | {
        layerId?: undefined;
        featureId?: undefined;
      }
    | {
        layerId: string;
        featureId: string;
      };
  pluginSketchLayerFeatureRemove: (layer: LazyLayer, featureId: string) => void;
  onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
  onSketchFeatureUpdate?: (feature: SketchFeature) => void;
  onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
  onSketchPluginFeatureCreate?: (props: SketchEventProps) => void;
  onSketchPluginFeatureUpdate?: (props: SketchEventProps) => void;
  onSketchPluginFeatureDelete?: (props: { layerId: string; featureId: string }) => void;
  onLayerSelect?: OnLayerSelectType;
};

export default ({
  sketchOptions,
  from,
  updateType,
  layersRef,
  onSketchFeatureCreate,
  pluginSketchLayerCreate,
  pluginSketchLayerFeatureAdd,
  pluginSketchLayerFeatureUpdate,
  pluginSketchLayerFeatureRemove,
  onSketchPluginFeatureCreate,
  onSketchPluginFeatureUpdate,
  onSketchPluginFeatureDelete,
  onSketchFeatureUpdate,
  onSketchFeatureDelete,
  onLayerSelect,
}: Props) => {
  const handleFeatureCreate = useCallback(
    (feature: SketchFeature) => {
      if (sketchOptions.autoResetInteractionMode) {
        updateType(undefined);
      }

      if (from === "editor" && sketchOptions.dataOnly) {
        onSketchFeatureCreate?.(feature);
        return;
      }

      if (!sketchOptions.dataOnly) {
        const selectedLayer = layersRef.current?.selectedLayer();
        const { layerId, featureId } =
          !layersRef?.current?.isTempLayer(selectedLayer?.id) ||
          selectedLayer?.type !== "simple" ||
          selectedLayer.computed?.layer.type !== "simple"
            ? pluginSketchLayerCreate(feature)
            : pluginSketchLayerFeatureAdd(selectedLayer, feature);

        if (layerId && featureId) {
          requestAnimationFrame(() => {
            onLayerSelect?.(
              layerId,
              featureId,
              layerId
                ? () =>
                    new Promise(resolve => {
                      // Wait until computed feature is ready
                      queueMicrotask(() => {
                        resolve(layersRef.current?.findById?.(layerId)?.computed);
                      });
                    })
                : undefined,
              undefined,
              undefined,
            );
          });

          onSketchPluginFeatureCreate?.({ layerId, featureId, feature });
        }
      } else {
        onSketchPluginFeatureCreate?.({ feature });
      }
    },
    [
      layersRef,
      from,
      sketchOptions.dataOnly,
      sketchOptions.autoResetInteractionMode,
      pluginSketchLayerCreate,
      pluginSketchLayerFeatureAdd,
      onSketchFeatureCreate,
      onSketchPluginFeatureCreate,
      onLayerSelect,
      updateType,
    ],
  );

  const handleFeatureUpdate = useCallback(
    (feature: SketchFeature) => {
      if (from === "editor" && sketchOptions.dataOnly) {
        onSketchFeatureUpdate?.(feature);
        return;
      }

      if (!sketchOptions.dataOnly) {
        const selectedLayer = layersRef.current?.selectedLayer();
        if (!selectedLayer) return;

        const { layerId, featureId } = pluginSketchLayerFeatureUpdate(selectedLayer, feature);

        if (layerId && featureId) {
          setTimeout(() => {
            layersRef.current?.selectFeatures([
              {
                layerId,
                featureId: [featureId],
              },
            ]);
          }, 20);

          onSketchPluginFeatureUpdate?.({ layerId, featureId, feature });
        }
      } else {
        onSketchPluginFeatureUpdate?.({ feature });
      }
    },
    [
      from,
      sketchOptions.dataOnly,
      layersRef,
      pluginSketchLayerFeatureUpdate,
      onSketchFeatureUpdate,
      onSketchPluginFeatureUpdate,
    ],
  );

  const handleFeatureDelete = useCallback(
    (layerId: string, featureId: string) => {
      if (from === "editor" && sketchOptions.dataOnly) {
        onSketchFeatureDelete?.(layerId, featureId);
        return;
      }

      if (!sketchOptions.dataOnly) {
        const layer = layersRef.current?.findById(layerId);
        if (!layer) return;
        pluginSketchLayerFeatureRemove(layer, featureId);
        layersRef.current?.selectFeatures([]);
      }
      onSketchPluginFeatureDelete?.({ layerId, featureId });
    },
    [
      from,
      sketchOptions.dataOnly,
      layersRef,
      pluginSketchLayerFeatureRemove,
      onSketchFeatureDelete,
      onSketchPluginFeatureDelete,
    ],
  );

  return {
    handleFeatureCreate,
    handleFeatureUpdate,
    handleFeatureDelete,
  };
};
