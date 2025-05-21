import { ClassificationType, Color, Rectangle } from "cesium";
import { FC, memo, useMemo } from "react";
import { Entity } from "resium";

type SpatialIdComponentProps = {
  wsen: [number, number, number, number];
  color: string;
};

export const CoordinateIndicator: FC<SpatialIdComponentProps> = memo(({ wsen, color }) => {
  const options = useMemo(
    () => ({
      rectangle: {
        coordinates: Rectangle.fromDegrees(...wsen),
        clampToGround: true,
        fill: true,
        outline: false,
        material: Color.fromCssColorString(color),
        classificationType: ClassificationType.BOTH,
      },
    }),
    [wsen, color],
  );

  return <Entity {...options} />;
});

CoordinateIndicator.displayName = "SpatialIdSpace";
