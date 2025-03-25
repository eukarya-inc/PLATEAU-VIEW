import { CameraPosition, XYZ } from "../types";

import { isReEarthAPIv2 } from "./reearth";

export const flyToLayerId = (layerId: string) =>
  window.reearth?.camera?.flyTo(layerId, { duration: 0.5 });

export const flyToCamera = (camera: CameraPosition, duration = 0.5) =>
  window.reearth?.camera?.flyTo(camera, { duration });

export const flyToBBox = (bbox: [number, number, number, number], duration = 0.5) => {
  const camera = window.reearth?.camera?.position;
  isReEarthAPIv2(window.reearth)
    ? window.reearth?.camera?.flyToBoundingBox(
        {
          west: bbox[0],
          south: bbox[1],
          east: bbox[2],
          north: bbox[3],
        },
        {
          duration,
          heading: camera?.heading,
          pitch: camera?.pitch,
        },
      )
    : window.reearth?.camera?.flyToBBox(bbox, {
        duration,
        heading: camera?.heading,
        pitch: camera?.pitch,
      });
};

export const setView = (camera: CameraPosition) => window.reearth?.camera?.setView(camera);

export const lookAtXYZ = ({ x, y, z, radius }: XYZ) => {
  const [lng, lat, height] = (isReEarthAPIv2(window.reearth)
    ? window.reearth?.viewer?.tools?.cartesianToCartographic(x, y, z, {
        useGlobeEllipsoid: true,
      })
    : window.reearth?.scene?.toLngLatHeight(x, y, z, {
        useGlobeEllipsoid: true,
      })) ?? [0, 0, 0];
  window.reearth?.camera?.lookAt(
    {
      lng,
      lat,
      height,
      radius,
    },
    { duration: 0.5 },
  );
};

export const lookAtTileFeature = (properties: Record<string, any> | undefined) => {
  if (!properties) return;
  const [x, y] = [properties["_x"], properties["_y"]];
  if (x === undefined || y === undefined) return;

  (isReEarthAPIv2(window.reearth)
    ? window.reearth?.viewer?.tools?.getTerrainHeightAsync?.(x, y)
    : window.reearth?.scene?.sampleTerrainHeight?.(x, y)
  )?.then((terrainHeight: number | undefined) => {
    window.reearth?.camera?.lookAt?.(
      {
        lng: x,
        lat: y,
        height:
          (Number(properties["計測高さ"] ?? properties["bldg:measuredHeight"]) || 100) +
          (terrainHeight ?? 0),
        range: 300,
      },
      { duration: 2 },
    );
  });
};
