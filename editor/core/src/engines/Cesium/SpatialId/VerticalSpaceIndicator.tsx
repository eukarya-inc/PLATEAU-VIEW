import { ClassificationType, Color, Rectangle } from "cesium";
import { FC, memo, useMemo } from "react";
import { Entity } from "resium";

import { VerticalSpaceIndicatorType } from "../../../Map/SpatialId/types";

type VerticalSpaceIndicatorProps = {
  indicator: VerticalSpaceIndicatorType;
};

export const VerticalSpaceIndicator: FC<VerticalSpaceIndicatorProps> = memo(({ indicator }) => {
  const options = useMemo(
    () => ({
      rectangle: {
        coordinates: Rectangle.fromDegrees(...indicator.wsen),
        height: indicator.height,
        extrudedHeight: indicator.extrudedHeight,
        clampToGround: false,
        fill: true,
        outline: true,
        outlineColor: Color.fromCssColorString(indicator.outlineColor),
        material: Color.fromCssColorString(indicator.color),
        classificationType: ClassificationType.TERRAIN,
      },
    }),
    [indicator],
  );

  return <Entity {...options} />;
});

VerticalSpaceIndicator.displayName = "VerticalSpaceIndicator";
