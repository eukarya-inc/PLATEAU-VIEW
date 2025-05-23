import { LineString, MultiPolygon, Point, Polygon, Feature } from "geojson";
import { Dispatch, MutableRefObject, SetStateAction, useCallback, useEffect, useRef } from "react";
import invariant from "tiny-invariant";
import { RefObject } from "use-callback-ref/dist/es5/types";

import { Position3d } from "../../types";
import { EngineRef, MouseEventCallback, MouseEventProps } from "../types";

import { SketchInterpreter, sketchState } from "./hooks";
import { Typegen0 } from "./sketchMachine.typegen";
import {
  GeometryOptionsXYZ,
  SketchEditingFeature,
  SketchFeature,
  SketchOptions,
  SketchType,
} from "./types";

type Props = {
  state: sketchState;
  send: SketchInterpreter["send"];
  engineRef: RefObject<EngineRef>;
  disableInteraction: boolean;
  type: SketchType | undefined;
  updateType: Dispatch<SetStateAction<SketchType | undefined>>;
  geometryOptions: GeometryOptionsXYZ | null;
  setGeometryOptions: Dispatch<SetStateAction<GeometryOptionsXYZ | null>>;
  updateGeometryOptions: (controlPoint?: Position3d) => void;
  sketchEditingFeature?: SketchEditingFeature;
  setSelectedControlPointIndex: Dispatch<SetStateAction<number | undefined>>;
  markerGeometryRef: MutableRefObject<GeometryOptionsXYZ | null>;
  pointerLocationRef: MutableRefObject<[lng: number, lat: number, height: number] | undefined>;
  extrudedHeight: number;
  setExtrudedHeight: (height: number) => void;
  setExtrudedPoint: (point: Position3d | undefined) => void;
  updateCentroidPoints: (controlPoints: Position3d[]) => void;
  createFeature: () => Feature<
    Polygon | MultiPolygon | Point | LineString,
    {
      id: string;
      type: SketchType;
      positions: Position3d[];
      extrudedHeight: number;
    }
  > | null;
  handleFeatureCreate: (feature: SketchFeature) => void;
  applyEdit: () => void;
  cancelEdit: () => void;
  isEditing: boolean;
  sketchOptions: SketchOptions;
};

const movingStatus: Typegen0["matchesStates"][] = [
  "editing.marker.moving",
  "editing.polyline.moving",
  "editing.polygon.moving",
  "editing.circle.moving",
  "editing.rectangle.moving",
  "editing.extrudedCircle.moving",
  "editing.extrudedRectangle.moving",
  "editing.extrudedPolygon.moving",
];

export default ({
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
}: Props) => {
  const handleLeftDown = useCallback(
    (props: MouseEventProps) => {
      setSelectedControlPointIndex(undefined);

      if (
        disableInteraction ||
        (!type && !sketchEditingFeature) ||
        props.lng === undefined ||
        props.lat === undefined ||
        props.height === undefined ||
        props.x === undefined ||
        props.y === undefined
      ) {
        return;
      }

      if (state.matches("idle") && type) {
        invariant(state.context.lastControlPoint == null);
        const controlPoint = engineRef.current?.toXYZ(props.lng, props.lat, props.height);
        if (controlPoint == null) {
          return;
        }

        send({
          type: {
            marker: "MARKER" as const,
            polyline: "POLYLINE" as const,
            circle: "CIRCLE" as const,
            rectangle: "RECTANGLE" as const,
            polygon: "POLYGON" as const,
            extrudedCircle: "EXTRUDED_CIRCLE" as const,
            extrudedRectangle: "EXTRUDED_RECTANGLE" as const,
            extrudedPolygon: "EXTRUDED_POLYGON" as const,
          }[type],
          pointerPosition: [props.x, props.y],
          controlPoint,
        });
        setGeometryOptions(null);
        markerGeometryRef.current = null;
      }
    },
    [
      state,
      disableInteraction,
      type,
      sketchEditingFeature,
      engineRef,
      send,
      setGeometryOptions,
      setSelectedControlPointIndex,
      markerGeometryRef,
    ],
  );

  const handleMouseMove = useCallback(
    (props: MouseEventProps) => {
      if (
        disableInteraction ||
        props.lng === undefined ||
        props.lat === undefined ||
        props.height === undefined ||
        props.x === undefined ||
        props.y === undefined ||
        !engineRef.current
      ) {
        return;
      }

      pointerLocationRef.current = [props.lng, props.lat, props.height];

      if (state.matches("drawing")) {
        invariant(state.context.type != null);
        invariant(state.context.controlPoints != null);
        let controlPoint = engineRef.current?.toXYZ(props.lng, props.lat, props.height);

        if (
          controlPoint == null ||
          hasDuplicate(engineRef.current.equalsEpsilon3d, controlPoint, state.context.controlPoints)
        ) {
          return;
        }

        if (
          (state.context.type === "rectangle" || state.context.type === "extrudedRectangle") &&
          state.context.controlPoints.length === 2 &&
          controlPoint
        ) {
          const fixedControlPoints = engineRef.current.calcRectangleControlPoint(
            state.context.controlPoints[0],
            state.context.controlPoints[1],
            controlPoint,
          );
          controlPoint = fixedControlPoints[2];
        }
        updateGeometryOptions(controlPoint);
      } else if (state.matches("extruding")) {
        invariant(geometryOptions?.controlPoints != null);
        let extrudedHeight;
        if (state.context.type !== "extrudedRectangle") {
          extrudedHeight = engineRef.current?.getExtrudedHeight(
            geometryOptions?.controlPoints[geometryOptions?.controlPoints?.length - 1],
            [props.x, props.y],
          );
        } else if (state.context.originalControlPoint) {
          extrudedHeight = engineRef.current?.getExtrudedHeight(
            state.context.originalControlPoint,
            [props.x, props.y],
          );
        }
        if (extrudedHeight != null) {
          setExtrudedHeight(extrudedHeight);
          if (state.context.controlPoints) {
            const extrudePoint = engineRef.current?.getExtrudedPoint(
              geometryOptions?.controlPoints[geometryOptions?.controlPoints?.length - 1],
              extrudedHeight,
            );
            setExtrudedPoint(extrudePoint);
          }
        }
      } else if (movingStatus.some(state.matches)) {
        if (state.context.catchedExtrudedPoint) {
          invariant(geometryOptions?.controlPoints != null);
          const extrudedHeight = engineRef.current?.getExtrudedHeight(
            geometryOptions?.controlPoints[geometryOptions?.controlPoints?.length - 1],
            [props.x, props.y],
          );
          if (extrudedHeight != null) {
            setExtrudedHeight(extrudedHeight);
            if (state.context.controlPoints) {
              const extrudePoint = engineRef.current?.getExtrudedPoint(
                geometryOptions?.controlPoints[geometryOptions?.controlPoints?.length - 1],
                extrudedHeight,
              );
              setExtrudedPoint(extrudePoint);
            }
          }
        } else {
          const controlPoint = engineRef.current.toXYZ(props.lng, props.lat, props.height);

          if (
            controlPoint == null ||
            !state.context.controlPoints ||
            state.context.catchedControlPointIndex === -1 ||
            state.context.catchedControlPointIndex === undefined
          )
            return;

          let newControlPoints = state.context.controlPoints.toSpliced(
            state.context.catchedControlPointIndex,
            1,
            controlPoint,
          );

          if (
            (state.context.type === "rectangle" || state.context.type === "extrudedRectangle") &&
            state.context.controlPoints.length === 3
          ) {
            newControlPoints = engineRef.current.calcRectangleControlPoint(
              newControlPoints[0],
              newControlPoints[1],
              newControlPoints[2],
            );
          }

          if (
            ["extrudedCircle", "extrudedRectangle", "extrudedPolygon"].includes(
              state.context.type ?? "",
            )
          ) {
            const newExtrudedPoint = engineRef.current.getExtrudedPoint(
              newControlPoints[newControlPoints.length - 1],
              extrudedHeight,
            );
            setExtrudedPoint(newExtrudedPoint);

            if (sketchOptions.useCentroidExtrudedHeight) {
              updateCentroidPoints(state.context.controlPoints);
            }
          }

          send({
            type: "MOVE",
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
        }
      }
    },
    [
      geometryOptions,
      disableInteraction,
      state,
      engineRef,
      extrudedHeight,
      send,
      updateGeometryOptions,
      updateCentroidPoints,
      setExtrudedHeight,
      setExtrudedPoint,
      pointerLocationRef,
      setGeometryOptions,
      sketchOptions.useCentroidExtrudedHeight,
    ],
  );

  const handleLeftUp = useCallback(
    (props: MouseEventProps) => {
      if (
        disableInteraction ||
        props.lng === undefined ||
        props.lat === undefined ||
        props.height === undefined ||
        props.x === undefined ||
        props.y === undefined ||
        !engineRef.current
      ) {
        return;
      }

      if (movingStatus.some(state.matches)) {
        send({
          type: "RELEASE",
          controlPoints: state.context.controlPoints ?? [],
          catchedControlPointIndex: -1,
          catchedExtrudedPoint: false,
        });
        setGeometryOptions(op =>
          op
            ? {
                type: op.type,
                controlPoints: state.context.controlPoints ?? [],
              }
            : null,
        );
      }

      if (
        state.context.controlPoints?.length === 1 &&
        state.context.lastPointerPosition != null &&
        state.context.type !== "marker" &&
        engineRef.current?.equalsEpsilon2d(
          [props.x, props.y],
          state.context.lastPointerPosition,
          0,
          5,
        )
      ) {
        return; // Too close to the first position user clicked.
      }

      if (state.matches("drawing")) {
        const controlPoint = engineRef.current?.toXYZ(props.lng, props.lat, props.height);
        if (controlPoint == null) return;

        if (state.context.type === "marker") {
          markerGeometryRef.current = {
            type: state.context.type,
            controlPoints: [controlPoint],
          };
          const feature = createFeature();
          markerGeometryRef.current = null;
          if (feature == null) {
            return;
          }
          handleFeatureCreate(feature);
          send({ type: "CREATE" });
          setGeometryOptions(null);
          return;
        }
        if (
          hasDuplicate(
            engineRef.current?.equalsEpsilon3d,
            controlPoint,
            state.context.controlPoints,
          )
        ) {
          return;
        }
        if (
          state.context.type === "circle" ||
          (state.context.type === "rectangle" && state.context.controlPoints?.length === 2)
        ) {
          const feature = createFeature();
          if (feature == null) {
            return;
          }
          handleFeatureCreate(feature);
          send({ type: "CREATE" });
          setGeometryOptions(null);
          return;
        } else {
          if (props.x === undefined || props.y === undefined) return;
          send({
            type: "NEXT",
            pointerPosition: [props.x, props.y],
            controlPoint,
          });
        }
      } else if (state.matches("extruding")) {
        const feature = createFeature();
        if (feature == null) {
          return;
        }
        handleFeatureCreate(feature);
        send({ type: "CREATE" });
        setGeometryOptions(null);
      }
    },
    [
      disableInteraction,
      state,
      engineRef,
      send,
      setGeometryOptions,
      createFeature,
      handleFeatureCreate,
      markerGeometryRef,
    ],
  );

  const handleDoubleClick = useCallback(
    (props: MouseEventProps) => {
      if (isEditing) {
        applyEdit();
        return;
      }

      if (
        disableInteraction ||
        props.lng === undefined ||
        props.lat === undefined ||
        props.height === undefined ||
        props.x === undefined ||
        props.y === undefined
      ) {
        return;
      }
      if (state.matches("drawing.extrudedPolygon")) {
        const controlPoint = engineRef.current?.toXYZ(props.lng, props.lat, props.height);
        if (controlPoint == null) return;
        send({
          type: "EXTRUDE",
          pointerPosition: [props.x, props.y],
          controlPoint,
        });
      } else if (state.matches("drawing.polyline") || state.matches("drawing.polygon")) {
        const feature = createFeature();
        if (feature == null) {
          return;
        }
        handleFeatureCreate(feature);
        send({ type: "CREATE" });
        setGeometryOptions(null);
      }
    },
    [
      isEditing,
      applyEdit,
      disableInteraction,
      state,
      engineRef,
      send,
      handleFeatureCreate,
      createFeature,
      setGeometryOptions,
    ],
  );

  const handleRightClick = useCallback(() => {
    if (isEditing) {
      cancelEdit();
      return;
    }
    if (!sketchOptions.rightClickToAbort) {
      return;
    }
    if (type !== undefined) {
      updateType(undefined);
    }
    if (state.matches("idle")) return;
    send({ type: "ABORT" });
    updateGeometryOptions(undefined);
  }, [
    isEditing,
    cancelEdit,
    type,
    state,
    sketchOptions.rightClickToAbort,
    send,
    updateGeometryOptions,
    updateType,
  ]);

  const mouseDownEventRef = useRef<MouseEventCallback>(handleLeftDown);
  mouseDownEventRef.current = handleLeftDown;
  const mouseMoveEventRef = useRef<MouseEventCallback>(handleMouseMove);
  mouseMoveEventRef.current = handleMouseMove;
  const mouseUpEventRef = useRef<MouseEventCallback>(handleLeftUp);
  mouseUpEventRef.current = handleLeftUp;
  const mouseDoubleClickEventRef = useRef<MouseEventCallback>(handleDoubleClick);
  mouseDoubleClickEventRef.current = handleDoubleClick;
  const mouseRightClickEventRef = useRef<() => void>(handleRightClick);
  mouseRightClickEventRef.current = handleRightClick;

  const onMouseDown = useCallback(
    (props: MouseEventProps) => {
      mouseDownEventRef.current?.(props);
    },
    [mouseDownEventRef],
  );

  const onMouseMove = useCallback(
    (props: MouseEventProps) => {
      mouseMoveEventRef.current?.(props);
    },
    [mouseMoveEventRef],
  );

  const onMouseUp = useCallback(
    (props: MouseEventProps) => {
      mouseUpEventRef.current?.(props);
    },
    [mouseUpEventRef],
  );

  const onMouseDoubleClick = useCallback(
    (props: MouseEventProps) => {
      mouseDoubleClickEventRef.current?.(props);
    },
    [mouseDoubleClickEventRef],
  );

  const onMouseRightClick = useCallback(() => {
    mouseRightClickEventRef.current?.();
  }, [mouseRightClickEventRef]);

  const eventsBinded = useRef(false);

  useEffect(() => {
    if (eventsBinded.current || !engineRef.current) return;
    eventsBinded.current = true;
    engineRef.current?.onMouseDown(onMouseDown);
    engineRef.current?.onMouseMove(onMouseMove);
    engineRef.current?.onMouseUp(onMouseUp);
    engineRef.current?.onDoubleClick(onMouseDoubleClick);
    engineRef.current?.onRightClick(onMouseRightClick);
  }, [engineRef, onMouseDown, onMouseMove, onMouseUp, onMouseDoubleClick, onMouseRightClick]);

  return {};
};

function hasDuplicate(
  equalFunction: (
    point1: Position3d,
    point2: Position3d,
    relativeEpsilon: number | undefined,
    absoluteEpsilon: number | undefined,
  ) => boolean,
  controlPoint: Position3d,
  controlPoints?: readonly Position3d[],
): boolean {
  return controlPoints?.some(another => equalFunction(controlPoint, another, 0, 1e-7)) === true;
}
