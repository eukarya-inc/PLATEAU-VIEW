import { Entity, PointGraphics, BillboardGraphics, JulianDate } from "cesium";

import { EvalFeature } from "../../..";
import { AppearanceTypes, ComputedFeature, ComputedLayer, Feature } from "../../../../mantle";
import { heightReference, shadowMode, toColor, classificationType } from "../../common";
import { getMarkerCoordinates, getGeometryFromEntity } from "../../helpers/getGeometryFromEntity";
import { convertEntityDescription, convertEntityProperties } from "../../utils/utils";
import { attachTag, extractSimpleLayer, getTag, Tag } from "../utils";

export function overrideOriginalProperties(
  entity: Entity,
  tag: Tag | undefined,
  name: string,
  properties: any,
) {
  const originalProperties = tag?.originalProperties || {};
  attachTag(entity, {
    ...(tag || {}),
    originalProperties: {
      ...originalProperties,
      [name]: {
        ...originalProperties[name],
        ...properties,
      },
    },
  });
}

type CesiumEntityAppearanceKey = "polygon" | "polyline";
type SupportedAppearanceKey = "marker" | keyof Pick<Entity, CesiumEntityAppearanceKey>;

type EntityAppearanceKey<AName extends SupportedAppearanceKey> = AName extends "marker"
  ? keyof Pick<Entity, "point" | "billboard" | "label">
  : keyof Pick<Entity, CesiumEntityAppearanceKey>;

type AppearancePropertyKeyType = "color" | "heightReference" | "shadows" | "classificationType";

export function attachProperties<
  AName extends SupportedAppearanceKey,
  PName extends EntityAppearanceKey<AName>,
>(
  entity: Entity,
  computedFeature: ComputedFeature | undefined,
  namePair: [appearanceName: AName, propertyName: PName],
  propertyMap: {
    [K in keyof Exclude<Entity[PName], undefined>]?: {
      name: keyof AppearanceTypes[AName];
      type?: AppearancePropertyKeyType;
      override?: any;
      default?: any;
    };
  },
) {
  const [appearanceName, propertyName] = namePair;
  const property = entity[propertyName] ?? {};
  if (!entity[propertyName]) {
    return;
  }

  const tag = getTag(entity);
  const originalProperties = tag?.originalProperties || {};
  const isUpdatedInLastUpdate = originalProperties[appearanceName];

  let isUpdated = false;
  Object.entries(propertyMap).forEach(([entityPropertyKey, appearancePropertyKey]) => {
    const appearanceKeyName = appearancePropertyKey.name;
    const appearanceKeyType = appearancePropertyKey.type as AppearancePropertyKeyType;

    let value =
      appearancePropertyKey.override ??
      (computedFeature?.[appearanceName] as any)?.[appearanceKeyName];
    const isDefaultUsed = value == null && !!appearancePropertyKey.default;
    value = value == null ? appearancePropertyKey.default : value;

    if ((value == null || isDefaultUsed) && !isUpdatedInLastUpdate) {
      return;
    }

    isUpdated = true;

    switch (appearanceKeyType) {
      case "color":
        value = toColor(value);
        break;
      case "shadows":
        value = shadowMode(value);
        break;
      case "heightReference":
        value = heightReference(value);
        break;
      case "classificationType":
        value = classificationType(value);
    }

    if (value === (entity[propertyName] as any)[entityPropertyKey]) {
      return;
    }

    (entity[propertyName] as any)[entityPropertyKey] =
      value ?? (property as any)[entityPropertyKey];
  });
  overrideOriginalProperties(entity, tag, appearanceName, isUpdated);
}

const hasAppearance = <
  AName extends SupportedAppearanceKey,
  PName extends EntityAppearanceKey<AName>,
>(
  layer: ComputedLayer | undefined,
  entity: Entity,
  namePair: [appearanceName: AName, propertyName: PName],
): boolean => {
  return !!(extractSimpleLayer(layer)?.[namePair[0]] && entity[namePair[1]]);
};

export const makeFeatureId = (e: Entity) => String(e.id);

export const attachStyle = (
  entity: Entity,
  layer: ComputedLayer | undefined,
  evalFeature: EvalFeature,
  currentTime: JulianDate,
): [Feature, ComputedFeature] | void => {
  if (!layer) {
    return;
  }

  // TODO: make it DRY
  const point = hasAppearance(layer, entity, ["marker", "point"]);
  const billboard = hasAppearance(layer, entity, ["marker", "billboard"]);
  const label = hasAppearance(layer, entity, ["marker", "label"]);
  if (entity.point || entity.billboard || entity.label) {
    const coordinates = getMarkerCoordinates(entity, currentTime);
    const geometry = getGeometryFromEntity(currentTime, entity);

    const feature: Feature = {
      type: "feature",
      id: makeFeatureId(entity),
      geometry,
      properties: convertEntityProperties(currentTime, entity),
      metaData: {
        description: convertEntityDescription(currentTime, entity),
      },
      range: {
        x: coordinates[0],
        y: coordinates[1],
        z: coordinates[2],
      },
    };
    const computedFeature = evalFeature(layer.layer, feature);
    if (!computedFeature) {
      return;
    }
    if (point) {
      const isPointStyle = computedFeature?.marker?.style === "point";
      if (isPointStyle && !entity.point) {
        entity.point = new PointGraphics();
        entity.billboard = undefined;
      }

      attachProperties(entity, computedFeature, ["marker", "point"], {
        show: {
          name: "show",
          ...(computedFeature?.marker?.style
            ? {
                override:
                  computedFeature?.marker?.style === "point" &&
                  (computedFeature?.marker.show ?? true),
              }
            : {}),
        },
        pixelSize: {
          name: "pointSize",
        },
        color: {
          name: "pointColor",
          type: "color",
        },
        outlineColor: {
          name: "pointOutlineColor",
          type: "color",
        },
        outlineWidth: {
          name: "pointOutlineWidth",
        },
        heightReference: {
          name: "heightReference",
          type: "heightReference",
        },
      });
    }

    if (billboard) {
      const isImageStyle = computedFeature?.marker?.style === "image";
      if (isImageStyle && !entity.billboard) {
        entity.billboard = new BillboardGraphics();
        entity.point = undefined;
      }

      attachProperties(entity, computedFeature, ["marker", "billboard"], {
        show: {
          name: "show",
          ...(computedFeature?.marker?.style
            ? {
                override:
                  computedFeature?.marker?.style === "image" &&
                  (computedFeature?.marker.show ?? true),
              }
            : {}),
        },
        image: {
          name: "image",
        },
        color: {
          name: "imageColor",
          type: "color",
        },
        scale: {
          name: "imageSize",
        },
        sizeInMeters: {
          name: "imageSizeInMeters",
        },
        heightReference: {
          name: "heightReference",
          type: "heightReference",
        },
        horizontalOrigin: {
          name: "imageHorizontalOrigin",
        },
        verticalOrigin: {
          name: "imageVerticalOrigin",
        },
      });

      if (label) {
        attachProperties(entity, computedFeature, ["marker", "label"], {
          show: {
            name: "show",
            default: true,
          },
          text: {
            name: "labelText",
          },
          backgroundColor: {
            name: "labelBackground",
            type: "color",
          },
          heightReference: {
            name: "heightReference",
            type: "heightReference",
          },
        });
      }
    }
    return [feature, computedFeature];
  }

  if (entity.polyline) {
    const entityPosition = entity.position?.getValue(currentTime);
    const geometry = getGeometryFromEntity(currentTime, entity);

    const feature: Feature = {
      type: "feature",
      id: makeFeatureId(entity),
      geometry,
      properties: convertEntityProperties(currentTime, entity),
      metaData: {
        description: convertEntityDescription(currentTime, entity),
      },
      range: {
        x: entityPosition?.x ?? 0,
        y: entityPosition?.y ?? 0,
        z: entityPosition?.z ?? 0,
      },
    };
    const computedFeature = evalFeature(layer.layer, feature);
    if (!computedFeature) {
      return;
    }
    if (hasAppearance(layer, entity, ["polyline", "polyline"])) {
      attachProperties(entity, computedFeature, ["polyline", "polyline"], {
        show: {
          name: "show",
          default: true,
        },
        width: {
          name: "strokeWidth",
        },
        material: {
          name: "strokeColor",
          type: "color",
        },
        shadows: {
          name: "shadows",
          type: "shadows",
        },
        clampToGround: {
          name: "clampToGround",
        },
        classificationType: {
          name: "classificationType",
          type: "classificationType",
        },
      });
    }
    return [feature, computedFeature];
  }

  if (entity.polygon) {
    const entityPosition = entity.position?.getValue(currentTime);
    const geometry = getGeometryFromEntity(currentTime, entity);

    const feature: Feature = {
      type: "feature",
      id: makeFeatureId(entity),
      geometry,
      properties: convertEntityProperties(currentTime, entity),
      metaData: {
        description: convertEntityDescription(currentTime, entity),
      },
      range: {
        x: entityPosition?.x ?? 0,
        y: entityPosition?.y ?? 0,
        z: entityPosition?.z ?? 0,
      },
    };
    const computedFeature = evalFeature(layer.layer, feature);
    if (!computedFeature) {
      return;
    }
    if (hasAppearance(layer, entity, ["polygon", "polygon"])) {
      attachProperties(entity, computedFeature, ["polygon", "polygon"], {
        show: {
          name: "show",
          default: true,
        },
        fill: {
          name: "fill",
        },
        material: {
          name: "fillColor",
          type: "color",
        },
        outline: {
          name: "stroke",
        },
        outlineColor: {
          name: "strokeColor",
          type: "color",
        },
        outlineWidth: {
          name: "strokeWidth",
        },
        shadows: {
          name: "shadows",
          type: "shadows",
        },
        heightReference: {
          name: "heightReference",
          type: "heightReference",
        },
        extrudedHeight: {
          name: "extrudedHeight",
        },
        classificationType: {
          name: "classificationType",
          type: "classificationType",
        },
      });
    }
    return [feature, computedFeature];
  }
};
