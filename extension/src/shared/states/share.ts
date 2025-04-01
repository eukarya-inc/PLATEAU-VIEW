import { atom } from "jotai";

import { isNotNullish } from "../../prototypes/type-helpers";
import {
  HEATMAP_LAYER,
  HeatmapLayerModel,
  MESH_CODE_LAYER,
  MY_DATA_LAYER,
  PEDESTRIAN_LAYER,
  PedestrianLayerModel,
  SKETCH_LAYER,
  SPATIAL_ID_LAYER,
  STORY_LAYER,
  SharedHeatmapLayer,
  SharedPedestrianLayer,
  SharedSketchLayer,
  SketchLayerModel,
} from "../../prototypes/view-layers";
import { isReEarthAPIv2 } from "../reearth/utils/reearth";
import { getSharedStoreValue, setSharedStoreValue } from "../sharedAtoms/store";
import { generateID } from "../utils";
import {
  MyDataLayerModel,
  SharedMyDataLayer,
  SharedStoryLayer,
  StoryLayerModel,
} from "../view-layers";
import { MeshCodeLayerModel, SharedMeshCodeLayer } from "../view-layers/meshCode";
import { SharedSpatialIdLayer, SpatialIdLayerModel } from "../view-layers/spatialId";

import { rootLayersAtom } from "./rootLayer";
import { sharedInitialCameraAtom, sharedInitialClockAtom } from "./scene";

// This is necessary to identify the shared state.
export const SHARED_PROJECT_ID = generateID();
export const SHARED_PROJECT_ID_KEY = "sharedProjectId";

export const shareAtom = atom(undefined, async (_get, set) => {
  await set(sharedInitialClockAtom, async () =>
    isReEarthAPIv2(window.reearth)
      ? window.reearth?.timeline?.currentTime?.getTime()
      : window.reearth?.clock?.currentTime?.getTime(),
  );
  await set(sharedInitialCameraAtom, async () => window.reearth?.camera?.position);
  await set(shareRootLayerAtom);
  await setSharedStoreValue(
    SHARED_PROJECT_ID_KEY,
    (await getSharedStoreValue(SHARED_PROJECT_ID_KEY)) ?? SHARED_PROJECT_ID,
  );
});

export type SharedRootLayer = (
  | {
      type: "dataset";
      datasetId: string;
      dataId: string | undefined;
      groupId: string | undefined;
    }
  | SharedHeatmapLayer
  | SharedPedestrianLayer
  | SharedMyDataLayer
  | SharedSketchLayer
  | SharedSpatialIdLayer
  | SharedMeshCodeLayer
  | SharedStoryLayer
) & { hidden?: boolean };

// For share feature
const SHARED_LAYERS_KEY = "$sharedLayers";
const shareRootLayerAtom = atom(undefined, async get => {
  const rootLayers: SharedRootLayer[] = get(rootLayersAtom)
    .map((r): SharedRootLayer | undefined => {
      switch (r.type) {
        case "dataset":
          return {
            type: "dataset",
            datasetId: r.id,
            dataId: get(r.currentDataIdAtom),
            groupId: get(r.currentGroupIdAtom),
            hidden: get(get(get(r.rootLayerAtom).layer).hiddenAtom),
          };
        case "layer": {
          const rootLayer = get(r.rootLayerAtom);
          const layer = get(rootLayer.layer);
          switch (layer.type) {
            case HEATMAP_LAYER: {
              const l = layer as HeatmapLayerModel;
              return {
                type: "heatmap",
                id: l.id,
                datasetId: l.datasetId,
                dataId: l.dataId,
                hidden: get(l.hiddenAtom),
              };
            }
            case PEDESTRIAN_LAYER: {
              const l = layer as PedestrianLayerModel;
              return {
                type: "pedestrian",
                id: l.id,
                hidden: get(l.hiddenAtom),
              };
            }
            case MY_DATA_LAYER: {
              const l = layer as MyDataLayerModel;
              return {
                type: "myData",
                id: l.id,
                title: l.title,
                url: l.url,
                format: l.format,
                layers: l.layers,
                csv: l.csv,
                hidden: get(l.hiddenAtom),
              };
            }
            case SKETCH_LAYER: {
              const l = layer as SketchLayerModel;
              return {
                type: "sketch",
                id: l.id,
                title: l.title,
                features: get(l.featuresAtom),
                hidden: get(l.hiddenAtom),
              };
            }
            case SPATIAL_ID_LAYER: {
              const l = layer as SpatialIdLayerModel;
              return {
                type: "spatialId",
                id: l.id,
                title: l.title,
                features: get(l.featuresAtom),
                hidden: get(l.hiddenAtom),
              };
            }
            case MESH_CODE_LAYER: {
              const l = layer as MeshCodeLayerModel;
              return {
                type: "meshCode",
                id: l.id,
                title: l.title,
                meshCodeLevel: l.meshCodeLevel,
                features: get(l.featuresAtom),
                hidden: get(l.hiddenAtom),
              };
            }
            case STORY_LAYER: {
              const l = layer as StoryLayerModel;
              return {
                type: "story",
                id: l.id,
                title: l.title,
                captures: get(l.capturesAtom),
                hidden: get(l.hiddenAtom),
              };
            }
          }
        }
      }
    })
    .filter(isNotNullish);
  await setSharedStoreValue(SHARED_LAYERS_KEY, rootLayers);
});
export const getSharedRootLayersAtom = atom(undefined, () => {
  return getSharedStoreValue<SharedRootLayer[]>(SHARED_LAYERS_KEY);
});
