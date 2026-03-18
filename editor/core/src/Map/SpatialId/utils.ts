import { v4 as uuid } from "uuid";

import { Space } from "@reearth/spatial-id-sdk";

import { SpatialIdSpaceType, SpatialIdSpaceData } from "./types";

const getRectangeParamsFromSpace = (space: Space) => {
  const vertices = space.vertices3d();
  const wsen: [number, number, number, number] = [
    vertices[0][0],
    vertices[1][1],
    vertices[2][0],
    vertices[3][1],
  ];
  const height = vertices[0][2];
  const extrudedHeight = vertices[4][2];
  return { wsen, height, extrudedHeight };
};

export const createSpatialIdSpace = (
  lng: number,
  lat: number,
  alt: number,
  zoom: number,
  geoidHeight: number,
): SpatialIdSpaceType => {
  const space = new Space({ lat, lng, alt }, zoom);
  const { wsen, height, extrudedHeight } = getRectangeParamsFromSpace(space);

  return {
    id: uuid(),
    space,
    wsen,
    height: height + geoidHeight,
    extrudedHeight: extrudedHeight + geoidHeight,
  };
};

export const getSpaceData = (space: Space): SpatialIdSpaceData => {
  return {
    id: space.id,
    center: space.center,
    alt: space.alt,
    zoom: space.zoom,
    zfxy: space.zfxy,
    zfxyStr: space.zfxyStr,
    tilehash: space.tilehash,
    hilbertTilehash: space.hilbertTilehash,
    hilbertIndex: space.hilbertIndex.toString(),
    vertices: space.vertices3d(),
  };
};

export const getVerticalLimits = (
  maxHeigth: number,
  minHeight: number,
  zoom: number,
): { top: number; bottom: number } => {
  // lat/lng doesn't matter
  const topSpace = new Space({ lat: 0, lng: 0, alt: maxHeigth }, zoom);
  const bottomSpace = new Space({ lat: 0, lng: 0, alt: minHeight }, zoom);
  const { height } = getRectangeParamsFromSpace(topSpace);
  const { extrudedHeight } = getRectangeParamsFromSpace(bottomSpace);
  return { top: height, bottom: extrudedHeight };
};
