import { Feature as GeojsonFeature, MultiPolygon, Polygon, Point, LineString } from "geojson";
import { ComponentType } from "react";

import { SketchComponentProps } from "../../engines/Cesium/Sketch";
import { ComputedFeature, LayerAppearanceTypes } from "../../mantle";
import { Position3d } from "../../types";

export type SketchComponentType = ComponentType<SketchComponentProps>;

export type SketchType =
  | "marker"
  | "polyline"
  | "circle"
  | "rectangle"
  | "polygon"
  | "extrudedCircle"
  | "extrudedRectangle"
  | "extrudedPolygon";

export type SketchOptions = {
  color?: string;
  appearance?: SketchAppearance;
  dataOnly?: boolean;
  disableShadow?: boolean;
  rightClickToAbort?: boolean;
  autoResetInteractionMode?: boolean;
  useCentroidExtrudedHeight?: boolean;
};

export type GeometryOptionsXYZ = {
  type: SketchType;
  controlPoints: Position3d[];
};

export type SketchFeature = GeojsonFeature<
  Polygon | MultiPolygon | Point | LineString,
  {
    id: string;
    type: SketchType;
    positions: readonly Position3d[];
    extrudedHeight: number;
  }
>;

export type SketchEventProps = {
  layerId?: string;
  featureId?: string;
  feature?: SketchFeature;
};

export type SketchEventCallback = (event: SketchEventProps) => void;

export type SketchAppearance = Partial<LayerAppearanceTypes>;

export function isSketchType(value: unknown): value is SketchType {
  return (
    value === "marker" ||
    value === "polyline" ||
    value === "circle" ||
    value === "rectangle" ||
    value === "polygon" ||
    value === "extrudedCircle" ||
    value === "extrudedRectangle" ||
    value === "extrudedPolygon"
  );
}

export type SketchEditingFeature = {
  layerId: string;
  feature: ComputedFeature;
};

export type SketchEditFeatureChangeCb = (feature: SketchEditingFeature | undefined) => void;
