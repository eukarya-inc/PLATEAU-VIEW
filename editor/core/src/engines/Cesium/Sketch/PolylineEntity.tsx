import { CallbackProperty, ClassificationType, Color, type Cartesian3 } from "@cesium/engine";
import { useMemo, useRef, type FC } from "react";

import { useConstant } from "../../../utils";
import { useContext } from "../Feature/context";

import { DEFAULT_EDIT_COLOR } from "./constants";
import { Entity, type EntityProps } from "./Entity";

export interface PolylineEntityProps {
  dynamic?: boolean;
  positions: Cartesian3[];
  color?: Color;
  isEditing?: boolean;
}

export const PolylineEntity: FC<PolylineEntityProps> = ({
  dynamic = false,
  positions: positionsProp,
  color,
  isEditing,
}) => {
  const positionsRef = useRef(positionsProp);
  positionsRef.current = positionsProp;
  const positionsProperty = useConstant(
    () => new CallbackProperty(() => positionsRef.current, !dynamic),
  );
  const positions = dynamic ? positionsProperty : positionsProp;

  const options = useMemo(
    (): EntityProps => ({
      polyline: {
        positions,
        width: isEditing ? 1.5 : 1.5,
        material: isEditing ? Color.fromCssColorString(DEFAULT_EDIT_COLOR) : color,
        classificationType: ClassificationType.TERRAIN,
        clampToGround: true,
      },
    }),
    [color, positions, isEditing],
  );

  const { requestRender } = useContext();
  requestRender?.();

  return <Entity {...options} />;
};
