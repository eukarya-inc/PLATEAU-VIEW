import { SketchType } from "@reearth/core";

export const DEFAULT_TILE = "open_street_map";

export const DEFAULT_LAYERS: string[] = [];

export const DEFAULT_CAMERA = {
  fov: 1.0471975511965976,
  heading: 6.075482442126033,
  height: 4065.852019268935,
  lat: 35.608034008903225,
  lng: 139.7728554580092,
  pitch: -0.45804512978428535,
  roll: 6.2830631767616465,
};

export const TILES = [
  "default",
  "default_label",
  "default_road",
  "open_street_map",
  "esri_world_topo",
  "black_marble",
  "japan_gsi_standard",
];

export const SKETCH_TOOLS: SketchType[] = [
  "marker",
  "polyline",
  "circle",
  "rectangle",
  "polygon",
  "extrudedCircle",
  "extrudedRectangle",
  "extrudedPolygon",
];
