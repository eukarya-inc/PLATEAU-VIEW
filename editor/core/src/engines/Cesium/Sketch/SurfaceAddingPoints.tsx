import { Cartesian3 } from "@cesium/engine";
import { memo, type FC } from "react";

import { Position3d } from "../../../types";

import { ControlPoint } from "./ControlPoint";
import { type GeometryOptions } from "./createGeometry";

export interface SurfaceAddingPointsProps {
  geometryOptions: GeometryOptions;
  isEditing?: boolean;
  handleAddControlPoint?: (position: Position3d, index: number) => void;
}

const cartesianScratch1 = new Cartesian3();

const SurfaceAddingPoints: FC<SurfaceAddingPointsProps> = memo(
  ({ geometryOptions: { type, controlPoints }, isEditing, handleAddControlPoint }) => {
    if (!["polyline", "polygon", "extrudedPolygon"].includes(type)) return null;

    const addingPoints = [];

    for (let i = 0; i < controlPoints.length - 1; i++) {
      const nextPoint = controlPoints[i + 1];
      const midPoint = Cartesian3.midpoint(controlPoints[i], nextPoint, cartesianScratch1);
      addingPoints.push(midPoint.clone());
    }

    if (type === "polygon" || type === "extrudedPolygon") {
      const midPoint = Cartesian3.midpoint(
        controlPoints[controlPoints.length - 1],
        controlPoints[0],
        cartesianScratch1,
      );
      addingPoints.push(midPoint.clone());
    }

    return (
      <>
        {addingPoints.map((addingPoint, index) => (
          <ControlPoint
            key={index}
            position={addingPoint}
            isAddingPoint
            index={index}
            clampToGround
            isEditing={isEditing}
            handleControlPointMouseEvent={() =>
              handleAddControlPoint?.([addingPoint.x, addingPoint.y, addingPoint.z], index)
            }
          />
        ))}
      </>
    );
  },
);

SurfaceAddingPoints.displayName = "SurfaceAddingPoints";

export default SurfaceAddingPoints;
