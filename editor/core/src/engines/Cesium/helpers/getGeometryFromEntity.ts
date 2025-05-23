import { Cartesian3, Entity, JulianDate, PolygonHierarchy } from "cesium";

import { Geometry } from "../../../mantle";

export function getMarkerCoordinates(
  entity: Entity,
  currentTime: JulianDate,
): [number, number, number] {
  const position = entity.position?.getValue(currentTime) as Cartesian3 | undefined;
  return [position?.x ?? 0, position?.y ?? 0, position?.z ?? 0];
}

export function getGeometryFromEntity(currentTime: JulianDate, entity: Entity) {
  if (entity.point || entity.billboard || entity.label) {
    const coordinates = getMarkerCoordinates(entity, currentTime);

    const geometry: Geometry = {
      type: "Point",
      coordinates,
    };
    return geometry;
  }

  if (entity.polyline) {
    const positions = entity.polyline?.positions?.getValue(currentTime) as Cartesian3[];
    const coordinates = positions?.map(position => [
      position?.x ?? 0,
      position?.y ?? 0,
      position?.z ?? 0,
    ]);

    const geometry: Geometry = {
      type: "LineString",
      coordinates,
    };
    return geometry;
  }

  if (entity.polygon) {
    const hierarchy = entity.polygon?.hierarchy?.getValue(currentTime) as PolygonHierarchy;
    const coordinates = hierarchy?.positions?.map(position => [
      position?.x ?? 0,
      position?.y ?? 0,
      position?.z ?? 0,
    ]);

    const geometry: Geometry = {
      type: "Polygon",
      coordinates: [coordinates],
    };
    return geometry;
  }

  return undefined;
}
