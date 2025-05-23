import { feature } from "@turf/helpers";
import { useMachine } from "@xstate/react";
import { Feature as GeojsonFeature, MultiPolygon, Polygon, Point, LineString } from "geojson";
import { cloneDeep, merge } from "lodash-es";
import {
  ForwardedRef,
  RefObject,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { v4 as uuidv4 } from "uuid";
import { InterpreterFrom, StateFrom } from "xstate";

import { ControlPointMouseEventHandler } from "../../engines/Cesium/Sketch";
import { useWindowEvent } from "../../utils/use-window-event";
import { InteractionModeType } from "../../Visualizer/interactionMode";
import { Feature, EngineRef, LayersRef, SketchRef } from "../types";
import { useGet } from "../utils";

import { PRESET_APPEARANCE, PRESET_COLOR } from "./preset";
import { Position3d, createSketchMachine } from "./sketchMachine";
import {
  GeometryOptionsXYZ,
  SketchType,
  SketchFeature,
  SketchEventProps,
  SketchOptions,
  SketchEditingFeature,
  SketchEditFeatureChangeCb,
} from "./types";
import usePluginSketchLayer from "./usePluginSketchLayer";
import useSketch from "./useSketch";
import useSketchFeature from "./useSketchFeature";

import { OnLayerSelectType } from ".";

export type SketchFeatureCallback = (
  feature: GeojsonFeature<Polygon | MultiPolygon | Point | LineString> | null,
) => void;

type Props = {
  ref: ForwardedRef<SketchRef>;
  layersRef: RefObject<LayersRef>;
  engineRef: RefObject<EngineRef>;
  interactionMode: InteractionModeType;
  selectedFeature?: Feature;
  overrideInteractionMode?: (mode: InteractionModeType) => void;
  onSketchTypeChange?: (type: SketchType | undefined, from?: "editor" | "plugin") => void;
  onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
  onSketchPluginFeatureCreate?: (props: SketchEventProps) => void;
  onSketchFeatureUpdate?: (feature: SketchFeature) => void;
  onSketchPluginFeatureUpdate?: (props: SketchEventProps) => void;
  onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
  onSketchPluginFeatureDelete?: (props: { layerId: string; featureId: string }) => void;
  onLayerSelect?: OnLayerSelectType;
  sketchEditingFeature?: SketchEditingFeature;
  onSketchEditFeature?: (feature: SketchEditingFeature | undefined) => void;
  onMount?: () => void;
};

const sketchMachine = createSketchMachine();

export type sketchState = StateFrom<typeof sketchMachine>;
export type SketchInterpreter = InterpreterFrom<typeof sketchMachine>;

export default function ({
  ref,
  engineRef,
  layersRef,
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
}: Props) {
  const [state, send] = useMachine(sketchMachine);
  const [type, updateType] = useState<SketchType | undefined>();
  const [from, updateFrom] = useState<"editor" | "plugin">("editor");

  const setType = useCallback(
    (type: SketchType | undefined, from?: "editor" | "plugin") => {
      updateType(type);
      updateFrom(from ?? "editor");
      if (type) {
        engineRef.current?.setCursor("crosshair");
      }
    },
    [engineRef],
  );

  const [disableInteraction, setDisableInteraction] = useState(false);

  const [sketchOptions, setSketchOptions] = useState<SketchOptions>({
    color: PRESET_COLOR,
    appearance: PRESET_APPEARANCE,
    dataOnly: false,
    disableShadow: false,
    rightClickToAbort: true,
    autoResetInteractionMode: true,
    // NOTE: Centroid extrude is not finalized yet
    useCentroidExtrudedHeight: false,
  });

  const overrideOptions = useCallback((options: SketchOptions) => {
    setSketchOptions(prev => ({
      ...prev,
      ...options,
      appearance: merge(cloneDeep(prev.appearance), options.appearance),
    }));
  }, []);

  const [geometryOptions, setGeometryOptions] = useState<GeometryOptionsXYZ | null>(null);
  const [extrudedHeight, setExtrudedHeight] = useState(0);
  const [extrudedPoint, setExtrudedPoint] = useState<Position3d | undefined>();

  const [centroidBasePoint, setCentroidBasePoint] = useState<Position3d | undefined>();
  const [centroidExtrudedPoint, setCentroidExtrudedPoint] = useState<Position3d | undefined>();

  const [selectedControlPointIndex, setSelectedControlPointIndex] = useState<number | undefined>();
  const markerGeometryRef = useRef<GeometryOptionsXYZ | null>(null);
  const pointerLocationRef = useRef<[lng: number, lat: number, height: number]>();

  const isEditing = useMemo(() => state.matches("editing"), [state]);

  const createFeature = useCallback(() => {
    const geoOptions = type === "marker" ? markerGeometryRef.current : geometryOptions;
    if (geoOptions == null) {
      return null;
    }
    const geometry = engineRef.current?.createGeometry(geoOptions);
    if (geometry == null || (type !== "polyline" && geometry.type === "LineString")) {
      return null;
    }
    return feature(geometry, {
      id: uuidv4(),
      type: geoOptions.type,
      positions: geoOptions.controlPoints,
      extrudedHeight,
    });
  }, [extrudedHeight, geometryOptions, markerGeometryRef, type, engineRef]);

  const updateFeature = useCallback(() => {
    if (geometryOptions == null || !selectedFeature?.id) {
      return null;
    }
    const geometry = engineRef.current?.createGeometry(geometryOptions);
    if (geometry == null) {
      return null;
    }
    return feature(geometry, {
      id: selectedFeature?.id,
      type: geometryOptions?.type,
      positions: geometryOptions?.controlPoints,
      extrudedHeight,
    });
  }, [extrudedHeight, geometryOptions, selectedFeature, engineRef]);

  const updateGeometryOptions = useCallback(
    (controlPoint?: Position3d) => {
      setExtrudedHeight(0);
      setExtrudedPoint(undefined);
      if (state.context.type == null || state.context.controlPoints == null) {
        setGeometryOptions(null);
        return;
      }
      setGeometryOptions({
        type: state.context.type,
        controlPoints:
          controlPoint != null
            ? [...state.context.controlPoints, controlPoint]
            : state.context.controlPoints,
      });
    },
    [state, setGeometryOptions, setExtrudedHeight],
  );

  const updateGeometryOptionsRef = useRef(updateGeometryOptions);
  updateGeometryOptionsRef.current = updateGeometryOptions;

  const updateCentroidPoints = useCallback(
    async (controlPoints: Position3d[]) => {
      const newExtrudeBasePoint = await getCentroid(controlPoints, engineRef);
      setCentroidBasePoint(newExtrudeBasePoint);

      if (!newExtrudeBasePoint) return;
      const centroidExtrudedPoint = engineRef.current?.getExtrudedPoint(
        newExtrudeBasePoint,
        extrudedHeight,
      );
      setCentroidExtrudedPoint(centroidExtrudedPoint);
    },
    [engineRef, extrudedHeight],
  );

  const {
    pluginSketchLayerCreate,
    pluginSketchLayerFeatureAdd,
    pluginSketchLayerFeatureUpdate,
    pluginSketchLayerFeatureRemove,
  } = usePluginSketchLayer({
    layersRef,
    sketchOptions,
  });

  const { handleFeatureCreate, handleFeatureUpdate, handleFeatureDelete } = useSketchFeature({
    layersRef,
    sketchOptions,
    from,
    updateType,
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
  });

  const editFeature = useCallback(
    (feature: SketchEditingFeature | undefined) => {
      onSketchEditFeature?.(feature);

      if (!state.matches("idle") || !feature) return;

      const type = feature?.feature?.properties?.type as SketchType;
      send({
        type: {
          marker: "EDIT_MARKER" as const,
          polyline: "EDIT_POLYLINE" as const,
          circle: "EDIT_CIRCLE" as const,
          rectangle: "EDIT_RECTANGLE" as const,
          polygon: "EDIT_POLYGON" as const,
          extrudedCircle: "EDIT_EXTRUDED_CIRCLE" as const,
          extrudedRectangle: "EDIT_EXTRUDED_RECTANGLE" as const,
          extrudedPolygon: "EDIT_EXTRUDED_POLYGON" as const,
        }[type],
        controlPoints: feature?.feature?.properties?.positions,
        extrudedHeight: feature?.feature?.properties?.extrudedHeight,
      });
      setGeometryOptions({
        type,
        controlPoints: feature?.feature?.properties?.positions,
      });
      if (feature?.feature?.properties?.extrudedHeight) {
        setExtrudedHeight(feature.feature.properties.extrudedHeight);
        setExtrudedPoint(
          engineRef.current?.getExtrudedPoint(
            feature?.feature?.properties?.positions[
              feature?.feature?.properties?.positions.length - 1
            ],
            feature.feature.properties.extrudedHeight,
          ),
        );
      }
    },
    [engineRef, state, onSketchEditFeature, send],
  );

  const cancelEdit = useCallback(
    (ignoreAutoReSelect?: boolean) => {
      send({ type: "EXIT_EDIT" });
      updateGeometryOptions(undefined);
      onSketchEditFeature?.(undefined);
      if (ignoreAutoReSelect) {
        ignoreAutoReSelectRef.current = true;
      }
    },
    [onSketchEditFeature, send, updateGeometryOptions],
  );

  const applyEdit = useCallback(() => {
    if (sketchEditingFeature) {
      const feature = updateFeature();
      if (feature) {
        handleFeatureUpdate({ ...feature, id: feature.properties.id });
      }
    }
    send({ type: "EXIT_EDIT" });
    updateGeometryOptions(undefined);
    onSketchEditFeature?.(undefined);
  }, [
    sketchEditingFeature,
    send,
    updateGeometryOptions,
    handleFeatureUpdate,
    updateFeature,
    onSketchEditFeature,
  ]);

  const deleteFeature = useCallback(
    (layerId: string, featureId: string) => {
      handleFeatureDelete(layerId, featureId);
    },
    [handleFeatureDelete],
  );

  useSketch({
    state,
    engineRef,
    disableInteraction,
    type,
    updateType,
    sketchEditingFeature,
    setSelectedControlPointIndex,
    send,
    setGeometryOptions,
    markerGeometryRef,
    pointerLocationRef,
    geometryOptions,
    updateGeometryOptions,
    extrudedHeight,
    setExtrudedHeight,
    setExtrudedPoint,
    updateCentroidPoints,
    createFeature,
    handleFeatureCreate,
    applyEdit,
    cancelEdit,
    isEditing,
    sketchOptions,
  });

  useWindowEvent("keydown", event => {
    if (type === undefined) return;
    if (event.code === "Space") {
      tempSwitchToMoveMode.current = true;
      setDisableInteraction(true);
      overrideInteractionMode?.("move");
    } else {
      if (disableInteraction) {
        return;
      }
      if (event.key === "Escape") {
        send({ type: "CANCEL" });
        const controlPoint =
          pointerLocationRef.current != null
            ? engineRef.current?.toXYZ(...pointerLocationRef.current)
            : undefined;
        updateGeometryOptions(controlPoint);
      } else if (event.key === "Delete" && state.matches("idle") && selectedFeature?.id) {
        const selectedLayer = layersRef.current?.selectedLayer();
        if (selectedLayer && layersRef.current?.isTempLayer(selectedLayer?.id)) {
          pluginSketchLayerFeatureRemove(selectedLayer, selectedFeature.id);
        }
      }
    }
  });

  useWindowEvent("keyup", event => {
    if (type === undefined) return;
    if (event.code === "Space") {
      overrideInteractionMode?.("sketch");
      setDisableInteraction(false);
      if (tempSwitchToMoveMode.current) {
        tempSwitchToMoveMode.current = false;
      }
    }
  });

  useEffect(() => {
    if (type === undefined && !sketchEditingFeature) {
      send({ type: "ABORT" });
      updateGeometryOptionsRef.current?.(undefined);
    }
  }, [type, sketchEditingFeature, send]);

  const fromRef = useRef(from);
  fromRef.current = from;
  const overrideInteractionModeRef = useRef(overrideInteractionMode);
  overrideInteractionModeRef.current = overrideInteractionMode;
  const onSketchTypeChangeRef = useRef(onSketchTypeChange);
  onSketchTypeChangeRef.current = onSketchTypeChange;
  const interactionModeRef = useRef(interactionMode);
  interactionModeRef.current = interactionMode;

  useEffect(() => {
    if ((sketchEditingFeature || type) && interactionModeRef.current !== "sketch") {
      overrideInteractionModeRef.current?.("sketch");
    }
  }, [type, sketchEditingFeature]);

  const isEditingRef = useRef(isEditing);
  isEditingRef.current = isEditing;
  const cancelEditRef = useRef(cancelEdit);
  cancelEditRef.current = cancelEdit;

  useEffect(() => {
    onSketchTypeChangeRef.current?.(type, fromRef.current);
    if (isEditingRef.current) {
      cancelEditRef.current();
    }
    if (type === undefined) {
      overrideInteractionModeRef.current?.(
        interactionModeRef.current === "sketch" ? "default" : interactionModeRef.current,
      );
    }
  }, [type]);

  const typeRef = useRef(type);
  typeRef.current = type;
  useEffect(() => {
    if (tempSwitchToMoveMode.current) return;
    if (interactionMode !== "sketch") {
      if (isEditingRef.current) {
        cancelEditRef.current();
      }
    }
  }, [interactionMode, updateType]);

  // Edit
  const onEditFeatureChangeCbs = useRef<SketchEditFeatureChangeCb[]>([]);
  const onEditFeatureChange = useCallback((cb: SketchEditFeatureChangeCb) => {
    onEditFeatureChangeCbs.current.push(cb);
  }, []);
  const onEditFeatureChangeRef = useRef(onEditFeatureChange);
  onEditFeatureChangeRef.current = onEditFeatureChange;

  const lastSketchEditingFeature = useRef<SketchEditingFeature | undefined>(undefined);

  const catchedControlPointIndex = useMemo(
    () => state.context.catchedControlPointIndex,
    [state.context.catchedControlPointIndex],
  );

  const catchedExtrudedPoint = useMemo(
    () => !!state.context.catchedExtrudedPoint,
    [state.context.catchedExtrudedPoint],
  );

  const ignoreAutoReSelectRef = useRef(false);

  useEffect(() => {
    onEditFeatureChangeCbs.current.forEach(cb => {
      cb(sketchEditingFeature);
    });
    if (sketchEditingFeature) lastSketchEditingFeature.current = sketchEditingFeature;
    else {
      // Select the feature after editing
      if (ignoreAutoReSelectRef.current) {
        ignoreAutoReSelectRef.current = false;
        return;
      }
      layersRef.current?.selectFeatures([
        {
          layerId: undefined,
          featureId: [],
        },
      ]);
      setTimeout(() => {
        if (lastSketchEditingFeature.current) {
          layersRef.current?.selectFeatures([
            {
              layerId: lastSketchEditingFeature.current?.layerId,
              featureId: [lastSketchEditingFeature.current?.feature.id],
            },
          ]);
        }
        lastSketchEditingFeature.current = undefined;
      }, 50);
    }
  }, [layersRef, sketchEditingFeature, onEditFeatureChangeCbs]);

  const handleControlPointMouseEvent: ControlPointMouseEventHandler = useCallback(
    (index, isExtrudedPoint, eventType) => {
      if (!state.matches("editing") || !state.context.controlPoints) return;

      if (eventType === "mousedown") {
        if (isExtrudedPoint) {
          send({
            type: "CATCH",
            catchedControlPointIndex: -1,
            controlPoints: state.context.controlPoints,
            catchedExtrudedPoint: true,
          });
        } else {
          send({
            type: "CATCH",
            catchedControlPointIndex: index,
            controlPoints: state.context.controlPoints,
            catchedExtrudedPoint: false,
          });
        }
      } else {
        if (
          !isExtrudedPoint &&
          (((state.context.type === "polygon" || state.context.type === "extrudedPolygon") &&
            state.context.controlPoints.length > 3) ||
            (state.context.type === "polyline" && state.context.controlPoints.length > 2))
        ) {
          setSelectedControlPointIndex(index);
        }
      }
    },
    [state, send],
  );

  // API
  const getType = useGet(type);
  const getOptions = useGet(sketchOptions);

  useImperativeHandle(
    ref,
    () => ({
      getType,
      setType,
      getOptions,
      overrideOptions,
      editFeature,
      cancelEdit,
      applyEdit,
      deleteFeature,
      onEditFeatureChange: onEditFeatureChangeRef.current,
    }),
    [
      getType,
      setType,
      getOptions,
      overrideOptions,
      editFeature,
      deleteFeature,
      cancelEdit,
      applyEdit,
    ],
  );

  useEffect(() => {
    onMount?.();
  }, [onMount]);

  const handleDeleteControlPoint = useCallback(() => {
    if (selectedControlPointIndex !== undefined) {
      const newControlPoints = state.context.controlPoints?.toSpliced(selectedControlPointIndex, 1);
      if (!newControlPoints) return;
      send({
        type: "UPDATE",
        controlPoints: newControlPoints,
      });
      setGeometryOptions(op =>
        op
          ? {
              type: op.type,
              controlPoints: newControlPoints,
            }
          : null,
      );
      setSelectedControlPointIndex(undefined);
    }
  }, [selectedControlPointIndex, state.context.controlPoints, send, setGeometryOptions]);

  const handleDeleteControlPointRef = useRef(handleDeleteControlPoint);
  handleDeleteControlPointRef.current = handleDeleteControlPoint;

  const handleAddControlPoint = useCallback(
    (controlPoint: Position3d, index: number) => {
      if (state.context.controlPoints == null) return;
      const insertPosition = index + 1;
      const newControlPoints = state.context.controlPoints.toSpliced(
        insertPosition,
        0,
        controlPoint,
      );
      send({
        type: "UPDATE",
        controlPoints: newControlPoints,
      });
      setGeometryOptions(op =>
        op
          ? {
              type: op.type,
              controlPoints: newControlPoints,
            }
          : null,
      );
    },
    [state.context.controlPoints, send, setGeometryOptions],
  );

  //
  const tempSwitchToMoveMode = useRef(false);
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    return window.addEventListener("keydown", event => {
      if (event.code === "Space" && stateRef.current.matches("editing")) {
        tempSwitchToMoveMode.current = true;
        overrideInteractionMode?.("move");
      } else if (event.code === "Delete" && stateRef.current.matches("editing")) {
        handleDeleteControlPointRef.current();
      }
    });
  }, [overrideInteractionMode]);

  useEffect(() => {
    return window.addEventListener("keyup", event => {
      if (event.code === "Space" && tempSwitchToMoveMode.current) {
        overrideInteractionMode?.("sketch");
        tempSwitchToMoveMode.current = false;
      }
    });
  }, [overrideInteractionMode]);

  useEffect(() => {
    if (!isEditing) {
      overrideInteractionModeRef.current?.("default");
    }
  }, [isEditing]);

  return {
    state,
    isEditing,
    catchedControlPointIndex,
    catchedExtrudedPoint,
    extrudedHeight,
    extrudedPoint,
    centroidBasePoint,
    centroidExtrudedPoint,
    geometryOptions,
    color: sketchOptions.color,
    disableShadow: sketchOptions.disableShadow,
    selectedControlPointIndex,
    handleControlPointMouseEvent,
    handleAddControlPoint,
  };
}

async function getCentroid(
  controlPoints: readonly Position3d[],
  engineRef: RefObject<EngineRef>,
): Promise<Position3d | undefined> {
  let totalLat = 0;
  let totalLng = 0;

  controlPoints.forEach(controlPoint => {
    const p = engineRef.current?.toLngLatHeight(...controlPoint);
    if (!p) return;
    totalLng += p[0];
    totalLat += p[1];
  });

  const centroidLat = totalLat / controlPoints.length;
  const centroidLng = totalLng / controlPoints.length;
  const centroidHeight =
    (await engineRef.current?.sampleTerrainHeight(centroidLng, centroidLat)) ?? 0;

  return engineRef.current?.toXYZ(centroidLng, centroidLat, centroidHeight);
}
