import { Feature as GeojsonFeature } from "geojson";
import { RefObject, useCallback } from "react";

import { ComputedFeature } from "../Layer";
import { LayerSimple, LazyLayer } from "../Layers";
import { LayersRef } from "../types";

import { SketchFeature, SketchOptions } from "./types";

export default ({
  layersRef,
  sketchOptions,
}: {
  layersRef: RefObject<LayersRef>;
  sketchOptions: SketchOptions;
}) => {
  const pluginSketchLayerCreate = useCallback(
    (feature: SketchFeature) => {
      const newLayer = layersRef.current?.add({
        type: "simple",
        data: {
          type: "geojson",
          isSketchLayer: true,
          value: {
            type: "FeatureCollection",
            features: [{ ...feature, id: feature.properties.id }],
          },
          idProperty: "id",
        },
        ...sketchOptions.appearance,
      });
      return { layerId: newLayer?.id, featureId: feature.properties.id };
    },
    [layersRef, sketchOptions.appearance],
  );

  const pluginSketchLayerFeatureAdd = useCallback(
    (layer: LazyLayer, feature: SketchFeature) => {
      if (layer.type !== "simple") return {};
      layersRef.current?.override(layer.id, {
        data: {
          ...layer.data,
          type: "geojson",
          value: {
            type: "FeatureCollection",
            features: [
              ...((layer.computed?.layer as LayerSimple)?.data?.value?.features ?? []),
              { ...feature, id: feature.properties.id },
            ],
          },
        },
      });
      return { layerId: layer.id, featureId: feature.properties.id };
    },
    [layersRef],
  );

  const pluginSketchLayerFeatureUpdate = useCallback(
    (layer: LazyLayer, feature: SketchFeature) => {
      if (layer.type !== "simple") return {};
      layersRef.current?.override(layer.id, {
        data: {
          ...layer.data,
          type: "geojson",
          value: {
            type: "FeatureCollection",
            features: (layer.computed?.layer as LayerSimple)?.data?.value?.features?.map(
              (f: ComputedFeature) => (f.id === feature.id ? feature : f),
            ),
          },
        },
      });
      return { layerId: layer.id, featureId: feature.properties.id };
    },
    [layersRef],
  );

  const pluginSketchLayerFeatureRemove = useCallback(
    (layer: LazyLayer, featureId: string) => {
      if (layer.type !== "simple" || layer.computed?.layer.type !== "simple") return;
      layersRef.current?.override(layer.id, {
        data: {
          ...layer.data,
          type: "geojson",
          value: {
            type: "FeatureCollection",
            features: [
              ...(layer.computed?.layer?.data?.value?.features ?? []).filter(
                (feature: GeojsonFeature) => feature.id !== featureId,
              ),
            ],
          },
        },
      });
    },
    [layersRef],
  );

  return {
    pluginSketchLayerCreate,
    pluginSketchLayerFeatureAdd,
    pluginSketchLayerFeatureUpdate,
    pluginSketchLayerFeatureRemove,
  };
};
