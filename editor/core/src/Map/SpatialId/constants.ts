import { SpatialIdPickSpaceOptions } from "./types";

export const SPATIALID_DEFAULT_OPTIONS: Required<SpatialIdPickSpaceOptions> = {
  zoom: 20,
  maxHeight: 4000,
  minHeight: -1000,
  dataOnly: false,
  rightClickToExit: true,
  color: "#00bebe44",
  outlineColor: "#00bebe55",
  groundIndicatorColor: "#00000066",
  selectorColor: "#ff990099",
  selectorOutlineColor: "#ff9900aa",
  verticalSpaceIndicatorColor: "#ffffff33",
  verticalSpaceIndicatorOutlineColor: "#ffffff55",
};

export const SPATIALID_LATITUDE_RANGE = {
  min: -85.0511,
  max: 85.0511,
};
