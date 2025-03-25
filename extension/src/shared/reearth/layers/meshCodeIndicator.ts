import { FC, useMemo } from "react";

import { useLayer } from "../hooks";
import { Data } from "../types";

type Props = {
  coordinates?: [number, number][][];
};

const appearances = {
  polygon: {
    fill: true,
    fillColor: "#00bebe00",
    stroke: true,
    strokeWidth: 2,
    strokeColor: "#00bebe",
    heightReference: "clamp" as const,
  },
};

export const MeshCodeIndicator: FC<Props> = ({ coordinates }) => {
  const data: Data = useMemo(
    () => ({
      type: "geojson",
      value: {
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            geometry: {
              type: "Polygon",
              coordinates,
            },
          },
        ],
      },
    }),
    [coordinates],
  );

  useLayer({
    data,
    appearances,
    visible: true,
  });

  return null;
};
