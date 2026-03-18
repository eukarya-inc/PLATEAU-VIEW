import { Cartesian3, Color } from "@cesium/engine";
import { memo, type FC } from "react";

import { Position3d } from "../../../types";

import { DEFAULT_EDIT_COLOR } from "./constants";
import { ControlPoint } from "./ControlPoint";
import { type GeometryOptions } from "./createGeometry";
import { ExtrudedMeasurement } from "./ExtrudedMeasurement";

import { ControlPointMouseEventHandler } from ".";

export interface ExtrudedControlPointsProps {
  geometryOptions: GeometryOptions;
  extrudedHeight: number;
  extrudedPoint?: Position3d;
  centroidBasePoint?: Position3d;
  centroidExtrudedPoint?: Position3d;
  catchedExtrudedPoint?: boolean;
  color?: Color;
  isEditing?: boolean;
  handleControlPointMouseEvent?: ControlPointMouseEventHandler;
}

const ExtrudedControlPoints: FC<ExtrudedControlPointsProps> = memo(
  ({
    geometryOptions: { controlPoints, type },
    extrudedHeight,
    color,
    isEditing,
    extrudedPoint: extrudedPointPosition,
    centroidBasePoint: extrudeBasePointPosition,
    centroidExtrudedPoint: extrudeControlPointPosition,
    catchedExtrudedPoint,
    handleControlPointMouseEvent,
  }) => {
    const controlPoint = controlPoints[controlPoints.length - 1];

    const extrudedPoint = extrudedPointPosition
      ? new Cartesian3(...extrudedPointPosition)
      : undefined;

    const heightBasePoint = extrudeBasePointPosition
      ? new Cartesian3(...extrudeBasePointPosition)
      : undefined;

    const centroidExtrudedPoint = extrudeControlPointPosition
      ? new Cartesian3(...extrudeControlPointPosition)
      : undefined;

    return (
      <>
        {extrudedPoint && (
          <>
            <ControlPoint
              index={-1}
              position={extrudedPoint}
              isEditing={isEditing}
              isExtrudedControlPoint
              handleControlPointMouseEvent={handleControlPointMouseEvent}
            />
            {(!isEditing || catchedExtrudedPoint) && (
              <ExtrudedMeasurement
                a={controlPoint}
                b={extrudedPoint}
                extrudedHeight={extrudedHeight}
                color={isEditing ? Color.fromCssColorString(DEFAULT_EDIT_COLOR) : color}
                showLine={type !== "extrudedPolygon"}
              />
            )}
          </>
        )}
        {heightBasePoint && (
          <ControlPoint
            index={-1}
            position={heightBasePoint}
            isEditing={isEditing}
            isExtrudedControlPoint
          />
        )}
        {centroidExtrudedPoint && (
          <ControlPoint
            index={-1}
            position={centroidExtrudedPoint}
            isEditing={isEditing}
            isExtrudedControlPoint
          />
        )}
      </>
    );
  },
);

ExtrudedControlPoints.displayName = "ExtrudedControlPoints";

export default ExtrudedControlPoints;
