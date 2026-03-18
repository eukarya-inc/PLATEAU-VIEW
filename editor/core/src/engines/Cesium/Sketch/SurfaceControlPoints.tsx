import { Cartesian3, Color } from "@cesium/engine";
import { memo, type FC } from "react";

import { DEFAULT_EDIT_COLOR } from "./constants";
import { ControlPoint } from "./ControlPoint";
import { type GeometryOptions } from "./createGeometry";
import { SurfaceMeasurement } from "./SurfaceMeasurement";

import { ControlPointMouseEventHandler } from ".";

export interface SurfaceControlPointsProps {
  geometryOptions: GeometryOptions;
  color?: Color;
  isEditing?: boolean;
  selectedControlPointIndex?: number;
  catchedControlPointIndex?: number;
  handleControlPointMouseEvent?: ControlPointMouseEventHandler;
}

const cartesianScratch1 = new Cartesian3();

const SurfaceControlPoints: FC<SurfaceControlPointsProps> = memo(
  ({
    geometryOptions: { type, controlPoints },
    color,
    isEditing,
    catchedControlPointIndex,
    selectedControlPointIndex,
    handleControlPointMouseEvent,
  }) => {
    const measurements: { points: [Cartesian3, Cartesian3]; showLine: boolean }[] = [];

    if (controlPoints.length >= 2) {
      if (isEditing) {
        if (catchedControlPointIndex !== undefined && catchedControlPointIndex !== -1) {
          switch (type) {
            case "polyline":
              if (catchedControlPointIndex > 0) {
                measurements.push({
                  points: [
                    controlPoints[catchedControlPointIndex - 1],
                    controlPoints[catchedControlPointIndex],
                  ],
                  showLine: false,
                });
              }
              if (catchedControlPointIndex < controlPoints.length - 1) {
                measurements.push({
                  points: [
                    controlPoints[catchedControlPointIndex],
                    controlPoints[catchedControlPointIndex + 1],
                  ],
                  showLine: false,
                });
              }
              break;
            case "circle":
            case "extrudedCircle":
              if (catchedControlPointIndex !== 2) {
                measurements.push({
                  points: [controlPoints[0], controlPoints[1]],
                  showLine: true,
                });
              }
              break;
            case "rectangle":
            case "extrudedRectangle":
              if (catchedControlPointIndex <= 1) {
                measurements.push({
                  points: [controlPoints[0], controlPoints[1]],
                  showLine: false,
                });
              } else if (catchedControlPointIndex === 2) {
                const [p1, p2, p3] = controlPoints;
                const p4 = Cartesian3.midpoint(p1, p2, cartesianScratch1);
                measurements.push({ points: [p4, p3], showLine: true });
              }
              break;
            case "polygon":
            case "extrudedPolygon":
              measurements.push({
                points: [
                  controlPoints[catchedControlPointIndex],
                  catchedControlPointIndex === 0
                    ? controlPoints[controlPoints.length - 1]
                    : controlPoints[catchedControlPointIndex - 1],
                ],
                showLine: false,
              });
              measurements.push({
                points: [
                  controlPoints[catchedControlPointIndex],
                  catchedControlPointIndex === controlPoints.length - 1
                    ? controlPoints[0]
                    : controlPoints[catchedControlPointIndex + 1],
                ],
                showLine: false,
              });
              break;
            default:
              break;
          }
        }
      } else {
        switch (type) {
          case "rectangle" || "extrudedRectangle":
            if (controlPoints.length === 2) {
              measurements.push({
                points: controlPoints as [Cartesian3, Cartesian3],
                showLine: true,
              });
            } else if (controlPoints.length === 3) {
              const [p1, p2, p3] = controlPoints;
              const p4 = Cartesian3.midpoint(p1, p2, cartesianScratch1);
              measurements.push({ points: [p4, p3], showLine: true });
            }
            break;
          default:
            measurements.push({
              points: controlPoints.slice(-2) as [Cartesian3, Cartesian3],
              showLine: type === "circle",
            });
            break;
        }
      }
    }

    return (
      <>
        {controlPoints.map((controlPoint, index) => (
          <ControlPoint
            key={index}
            position={controlPoint}
            index={index}
            isSelected={selectedControlPointIndex === index}
            clampToGround
            isEditing={isEditing}
            handleControlPointMouseEvent={handleControlPointMouseEvent}
          />
        ))}
        {measurements.map(({ points, showLine }, index) => (
          <SurfaceMeasurement
            key={index}
            a={points[0]}
            b={points[1]}
            color={isEditing ? Color.fromCssColorString(DEFAULT_EDIT_COLOR) : color}
            showLine={showLine}
          />
        ))}
      </>
    );
  },
);

SurfaceControlPoints.displayName = "SurfaceControlPoints";

export default SurfaceControlPoints;
