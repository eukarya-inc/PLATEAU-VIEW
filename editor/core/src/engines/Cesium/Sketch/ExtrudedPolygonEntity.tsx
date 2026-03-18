import {
  CallbackProperty,
  ClassificationType,
  Color,
  ColorMaterialProperty,
  HeightReference,
  ShadowMode,
  type PolygonHierarchy,
} from "@cesium/engine";
import { useMemo, useRef, type FC } from "react";

import { useConstant } from "../../../utils";
import { useContext } from "../Feature/context";

import { DEFAULT_EDIT_COLOR } from "./constants";
import { Entity, type EntityProps } from "./Entity";

export interface ExtrudedPolygonEntityProps {
  id?: string;
  hierarchy: PolygonHierarchy;
  extrudedHeight: number;
  color?: Color;
  disableShadow?: boolean;
  isEditing?: boolean;
}

export const ExtrudedPolygonEntity: FC<ExtrudedPolygonEntityProps> = ({
  id,
  hierarchy: hierarchyProp,
  extrudedHeight: extrudedHeightProp,
  color,
  disableShadow = false,
  isEditing,
}) => {
  const hierarchyRef = useRef(hierarchyProp);
  hierarchyRef.current = hierarchyProp;
  const hierarchyProperty = useConstant(
    () => new CallbackProperty(() => hierarchyRef.current, false),
  );
  const hierarchy = hierarchyProperty;

  const extrudedHeightRef = useRef(extrudedHeightProp);
  extrudedHeightRef.current = extrudedHeightProp;
  const extrudedHeightProperty = useConstant(
    () => new CallbackProperty(() => extrudedHeightRef.current, false),
  );
  const extrudedHeight = extrudedHeightProperty;

  const options = useMemo(
    (): EntityProps => ({
      polygon: {
        hierarchy,
        heightReference: HeightReference.RELATIVE_TO_TERRAIN,
        extrudedHeight,
        extrudedHeightReference: HeightReference.RELATIVE_TO_TERRAIN,
        // extrudedHeightReference: HeightReference.NONE,
        fill: true,
        outline: true,
        outlineWidth: 1,
        outlineColor: isEditing
          ? Color.fromCssColorString(DEFAULT_EDIT_COLOR)
          : color?.withAlpha(1),
        material: new ColorMaterialProperty(isEditing ? color?.withAlpha(0.2) : color),
        classificationType: ClassificationType.TERRAIN,
        shadows: disableShadow ? ShadowMode.DISABLED : ShadowMode.ENABLED,
      },
    }),
    [extrudedHeight, disableShadow, hierarchy, color, isEditing],
  );

  const { requestRender } = useContext();
  requestRender?.();

  return <Entity id={id} {...options} />;
};
