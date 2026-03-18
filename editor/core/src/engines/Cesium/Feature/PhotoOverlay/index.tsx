import { Cartesian3 } from "cesium";
import { useMemo } from "react";
import nl2br from "react-nl2br";
import { BillboardGraphics } from "resium";

import type { LegacyPhotooverlayAppearance } from "../../..";
import { heightReference, ho, useIcon, vo } from "../../common";
import {
  EntityExt,
  toDistanceDisplayCondition,
  toTimeInterval,
  type FeatureComponentConfig,
  type FeatureProps,
} from "../utils";

import useHooks, { photoDuration, photoExitDuration } from "./hooks";
import defaultImage from "./primPhotoIcon.svg";

export type Props = FeatureProps<Property>;

export type Property = LegacyPhotooverlayAppearance;

export default function PhotoOverlay({
  id,
  isVisible,
  property,
  geometry,
  isSelected,
  layer,
  feature,
}: Props) {
  const coordinates = useMemo(
    () =>
      geometry?.type === "Point"
        ? geometry.coordinates
        : property?.location
          ? [property.location.lng, property.location.lat, property.height ?? 0]
          : undefined,
    [geometry?.coordinates, geometry?.type, property?.height, property?.location],
  );

  const {
    show = true,
    image,
    imageSize,
    imageHorizontalOrigin,
    imageVerticalOrigin,
    imageCrop,
    imageShadow,
    imageShadowColor,
    imageShadowBlur,
    imageShadowPositionX,
    imageShadowPositionY,
    heightReference: hr,
    camera,
    photoOverlayImage,
    photoOverlayDescription,
  } = property ?? {};

  const [canvas] = useIcon({
    image: image || defaultImage,
    imageSize: image ? imageSize : undefined,
    crop: image ? imageCrop : undefined,
    shadow: image ? imageShadow : undefined,
    shadowColor: image ? imageShadowColor : undefined,
    shadowBlur: image ? imageShadowBlur : undefined,
    shadowOffsetX: image ? imageShadowPositionX : undefined,
    shadowOffsetY: image ? imageShadowPositionY : undefined,
  });

  const pos = useMemo(
    () =>
      coordinates
        ? Cartesian3.fromDegrees(coordinates[0], coordinates[1], coordinates[2])
        : undefined,
    [coordinates],
  );

  const { photoOverlayImageTransiton, exitPhotoOverlay } = useHooks({
    camera,
    isSelected: isSelected && !!photoOverlayImage,
  });

  const availability = useMemo(() => toTimeInterval(feature?.interval), [feature?.interval]);
  const distanceDisplayCondition = useMemo(
    () => toDistanceDisplayCondition(property?.near, property?.far),
    [property?.near, property?.far],
  );

  return !isVisible || !show || !pos ? null : (
    <>
      <EntityExt
        id={id}
        position={pos}
        layerId={layer?.id}
        featureId={feature?.id}
        draggable
        properties={feature?.properties}
        availability={availability}>
        <BillboardGraphics
          image={canvas}
          horizontalOrigin={ho(imageHorizontalOrigin)}
          verticalOrigin={vo(imageVerticalOrigin)}
          heightReference={heightReference(hr)}
          distanceDisplayCondition={distanceDisplayCondition}
        />
      </EntityExt>
      {photoOverlayImageTransiton === "unmounted" ? null : (
        <div
          onClick={exitPhotoOverlay}
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: "rgba(0, 0, 0, 0.5)",
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            transition:
              photoOverlayImageTransiton === "entering" || photoOverlayImageTransiton === "exiting"
                ? `all ${photoOverlayImageTransiton === "exiting" ? photoExitDuration : photoDuration}s ease`
                : undefined,
            opacity:
              photoOverlayImageTransiton === "entering" || photoOverlayImageTransiton === "entered"
                ? 1
                : 0,
          }}>
          <img src={photoOverlayImage} style={PhotoStyle} />
          {photoOverlayDescription && (
            <p style={DescriptionStyle}>{nl2br(photoOverlayDescription)}</p>
          )}
        </div>
      )}
    </>
  );
}

const PhotoStyle = {
  maxWidth: "95%",
  maxHeight: "80%",
  boxShadow: "0 0 15px rgba(0, 0, 0, 1)",
};

const DescriptionStyle = {
  position: "absolute" as const,
  bottom: "10px",
  left: "20px",
  right: "20px",
  textAlign: "left" as const,
  userSelect: "none" as const,
  color: "rgba(0,0,0,.2)",
  fontFamily:
    "Noto Sans, hiragino sans, hiragino kaku gothic proN, -apple-system, BlinkMacSystem, sans-serif;",
  fontSize: "12px",
  fontWeight: "normal",
  lineHeight: "20px",
};

export const config: FeatureComponentConfig = {
  noLayer: true,
};
