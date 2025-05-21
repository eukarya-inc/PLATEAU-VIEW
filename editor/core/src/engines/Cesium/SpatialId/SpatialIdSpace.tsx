import { ClassificationType, Color, HeightReference, Rectangle } from "cesium";
import { FC, memo, useMemo } from "react";
import { Entity } from "resium";

import { SpatialIdSpaceType } from "../../../Map/SpatialId/types";

type SpatialIdComponentProps = {
  space: SpatialIdSpaceType;
};

export const SpatialIdSpace: FC<SpatialIdComponentProps> = memo(({ space }) => {
  const options = useMemo(
    () => ({
      rectangle: {
        coordinates: Rectangle.fromDegrees(...space.wsen),
        height: space.height,
        heightReference: HeightReference.NONE,
        extrudedHeight: space.extrudedHeight,
        clampToGround: false,
        fill: true,
        outline: true,
        outlineColor: Color.fromCssColorString(space.outlineColor ?? ""),
        material: Color.fromCssColorString(space.color ?? ""),
        classificationType: ClassificationType.TERRAIN,
      },
    }),
    [space],
  );

  return <Entity id={space.id} {...options} />;
});

SpatialIdSpace.displayName = "SpatialIdSpace";
