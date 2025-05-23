import { Space } from "@reearth/spatial-id-sdk";

export type SpatialIdSpaceType = {
  id: string;
  space: Space;
  wsen: [number, number, number, number];
  height: number;
  extrudedHeight: number;
  color?: string;
  outlineColor?: string;
};

export type VerticalSpaceIndicatorType = {
  id: string;
  wsen: [number, number, number, number];
  height: number;
  extrudedHeight: number;
  color: string;
  outlineColor: string;
};

export type CoordinateSelectorType = {
  id: string;
  uid: string;
  wsen: [number, number, number, number];
  color: string;
};

export type SpatialIdPickSpaceOptions = {
  zoom?: number;
  maxHeight?: number;
  minHeight?: number;
  dataOnly?: boolean;
  rightClickToExit?: boolean;
  color?: string;
  outlineColor?: string;
  groundIndicatorColor?: string;
  selectorColor?: string;
  selectorOutlineColor?: string;
  verticalSpaceIndicatorColor?: string;
  verticalSpaceIndicatorOutlineColor?: string;
};

export type SpatialIdRef = {
  pickSpace: (options?: SpatialIdPickSpaceOptions) => void;
  exitPickSpace: () => void;
  onSpacePick: (cb: (space: SpatialIdSpaceData) => void) => void;
};

export type SpatialIdSpacePickingState = "idle" | "coordinate" | "fetchingGeoid" | "floor";

export type SpatialIdSpaceData = {
  id: string;
  center: { lat: number; lng: number; alt?: number };
  alt: number;
  zoom: number;
  zfxy: {
    z: number;
    f: number;
    x: number;
    y: number;
  };
  zfxyStr: string;
  tilehash: string;
  hilbertTilehash: string;
  hilbertIndex: string;
  vertices: [number, number, number][];
};
