// Reference: Sketch feature is basically referenced from https://github.com/takram-design-engineering/plateau-view/blob/main/libs/sketch/src/DynamicSketchObject.tsx

import { Color } from "@cesium/engine";
import { Cartesian3 } from "cesium";
import { memo, useMemo, type FC } from "react";

import { SketchType } from "../../../Map/Sketch/types";
import { Position3d } from "../../../types";
import { convertGeometryToPositionsArray, convertPolygonToHierarchyArray } from "../utils/polygon";

import { DEFAULT_SKETCH_COLOR } from "./constants";
import { createGeometry, GeometryOptions } from "./createGeometry";
import ExtrudedControlPoints from "./ExtrudedControlPoints";
import { ExtrudedPolygonEntity } from "./ExtrudedPolygonEntity";
import { PolygonEntity } from "./PolygonEntity";
import { PolylineEntity } from "./PolylineEntity";
import SurfaceAddingPoints from "./SurfaceAddingPoints";
import SurfaceControlPoints from "./SurfaceControlPoints";

export type SketchComponentProps = {
  geometryOptions?: {
    type: SketchType;
    controlPoints: readonly Position3d[];
  } | null;
  extrudedHeight?: number;
  extrudedPoint?: Position3d;
  centroidBasePoint?: Position3d;
  centroidExtrudedPoint?: Position3d;
  disableShadow?: boolean;
  color?: string;
  isEditing?: boolean;
  catchedControlPointIndex?: number;
  catchedExtrudedPoint?: boolean;
  selectedControlPointIndex?: number;
  handleControlPointMouseEvent?: ControlPointMouseEventHandler;
  handleAddControlPoint?: (position: Position3d, index: number) => void;
};

export type ControlPointMouseEventHandler = (
  index: number,
  isExtrudedPoint: boolean,
  type: "mousedown" | "click",
) => void;

const SketchComponent: FC<SketchComponentProps> = memo(
  ({
    geometryOptions,
    extrudedHeight,
    disableShadow,
    color: stringColor,
    isEditing,
    extrudedPoint,
    centroidBasePoint,
    centroidExtrudedPoint,
    catchedControlPointIndex,
    catchedExtrudedPoint,
    selectedControlPointIndex,
    handleControlPointMouseEvent,
    handleAddControlPoint,
  }) => {
    const cartesianGeometryOptions: GeometryOptions | null = useMemo(
      () =>
        geometryOptions
          ? {
              ...geometryOptions,
              controlPoints: geometryOptions?.controlPoints.map(p => new Cartesian3(...p)),
            }
          : null,
      [geometryOptions],
    );

    const g = useMemo(
      () => (cartesianGeometryOptions ? createGeometry(cartesianGeometryOptions) : null),
      [cartesianGeometryOptions],
    );

    const { positionsArray, hierarchyArray } = useMemo(() => {
      if (g?.type === "Point") {
        return {};
      } else if (g?.type === "LineString") {
        return { positionsArray: convertGeometryToPositionsArray(g) };
      } else if (g != null) {
        return {
          positionsArray: convertGeometryToPositionsArray(g),
          hierarchyArray: convertPolygonToHierarchyArray(g),
        };
      }
      return {};
    }, [g]);

    const color = useMemo(
      () => Color.fromCssColorString(stringColor ?? DEFAULT_SKETCH_COLOR),
      [stringColor],
    );

    return (
      <>
        {positionsArray?.map((positions, index) => (
          <PolylineEntity
            key={index}
            dynamic
            positions={positions}
            color={color}
            isEditing={isEditing}
          />
        ))}
        {hierarchyArray?.map((hierarchy, index) => (
          <PolygonEntity key={index} dynamic hierarchy={hierarchy} color={color} />
        ))}
        {cartesianGeometryOptions != null && (!extrudedHeight || isEditing) && (
          <SurfaceControlPoints
            geometryOptions={cartesianGeometryOptions}
            color={color}
            isEditing={isEditing}
            catchedControlPointIndex={catchedControlPointIndex}
            selectedControlPointIndex={selectedControlPointIndex}
            handleControlPointMouseEvent={handleControlPointMouseEvent}
          />
        )}
        {cartesianGeometryOptions != null && isEditing && (
          <SurfaceAddingPoints
            geometryOptions={cartesianGeometryOptions}
            isEditing={isEditing}
            handleAddControlPoint={handleAddControlPoint}
          />
        )}
        {cartesianGeometryOptions != null && extrudedHeight && (
          <ExtrudedControlPoints
            geometryOptions={cartesianGeometryOptions}
            extrudedHeight={extrudedHeight}
            extrudedPoint={extrudedPoint}
            centroidBasePoint={centroidBasePoint}
            centroidExtrudedPoint={centroidExtrudedPoint}
            catchedExtrudedPoint={catchedExtrudedPoint}
            color={color}
            isEditing={isEditing}
            handleControlPointMouseEvent={handleControlPointMouseEvent}
          />
        )}
        {extrudedHeight &&
          hierarchyArray?.map((hierarchy, index) => (
            <ExtrudedPolygonEntity
              key={index}
              hierarchy={hierarchy}
              extrudedHeight={extrudedHeight}
              disableShadow={disableShadow}
              color={color}
              isEditing={isEditing}
            />
          ))}
      </>
    );
  },
);

SketchComponent.displayName = "SketchComponent";

export default SketchComponent;
