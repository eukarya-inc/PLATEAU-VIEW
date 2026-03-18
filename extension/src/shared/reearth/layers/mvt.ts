import { FC, useCallback, useEffect, useMemo, useState } from "react";

import { layerItemProps } from "../../layerContainers/general";
import { useLayer } from "../hooks";
import { CameraPosition, LayerAppearanceTypes } from "../types";
import { Data } from "../types/layer";

export type MVTAppearances = Partial<
  Pick<LayerAppearanceTypes, "raster" | "marker" | "polygon" | "polyline">
>;

export type MVTProps = {
  url: string;
  onLoad?: (layerId: string, camera: CameraPosition) => void;
  visible?: boolean;
  appearances?: MVTAppearances;
  layers?: string[];
  sortedLayers?: layerItemProps[];
};

const DEFAULT_APPEARNACES: Partial<LayerAppearanceTypes> = {
  raster: {},
  marker: {
    heightReference: "clamp",
  },
  polyline: {
    clampToGround: true,
  },
  polygon: {
    heightReference: "clamp",
  },
};

// TileJSON 3.0.0 spec: https://github.com/mapbox/tilejson-spec/blob/master/3.0.0/README.md
type TileJSON = {
  tilejson?: string;
  name?: string;
  description?: string;
  version?: string;
  minzoom?: number;
  maxzoom?: number;
  center?: [lng: number, lat: number, zoom: number];
  bounds?: [left: number, bottom: number, right: number, top: number];
  vector_layers?: { id: string; fields: Record<string, string> }[];
};

// FME metadata.json format (legacy)
type RawMVTMeta = {
  name: string;
  description: string;
  version: number;
  minzoom: number;
  maxzoom: number;
  center: `${number},${number},${number}`;
  bounds: `${number},${number},${number}`;
  type: "overlay";
  format: "pbr";
};

type MVTMeta = {
  name?: string;
  description?: string;
  minzoom?: number;
  maxzoom?: number;
  center: [lng: number, lat: number, z: number];
  bounds: [x: number, y: number, z: number];
};

export const MVTLayer: FC<MVTProps> = ({
  url,
  onLoad,
  visible,
  appearances,
  layers,
  sortedLayers,
}) => {
  const [meta, setMeta] = useState<MVTMeta | undefined>();
  useEffect(() => {
    const fetchMVTMeta = async () => {
      const mvtBaseURL = url.match(/(.+)(\/{z}\/{x}\/{y}.mvt)/)?.[1];
      if (!mvtBaseURL) return;

      // Try tilejson.json first (standard format)
      const tileJSON = await fetch(`${mvtBaseURL}/tilejson.json`)
        .then(d => d.json())
        .then(d => d as TileJSON)
        .catch(() => undefined);

      if (tileJSON?.center && tileJSON?.bounds) {
        setMeta({
          name: tileJSON.name,
          description: tileJSON.description,
          minzoom: tileJSON.minzoom,
          maxzoom: tileJSON.maxzoom,
          center: tileJSON.center,
          bounds: [tileJSON.bounds[0], tileJSON.bounds[1], tileJSON.bounds[2]],
        });
        return;
      }

      // Fallback to metadata.json (FME legacy format)
      const data = await fetch(`${mvtBaseURL}/metadata.json`)
        .then(d => d.json())
        .then(d => d as RawMVTMeta)
        .catch(() => undefined);
      if (!data) return;

      const center = data?.center?.split(",")?.map((s: string) => Number(s));
      if (!center || center.length < 2) return;
      const bounds = data?.bounds?.split(",")?.map((s: string) => Number(s));
      if (!bounds || bounds.length < 2) return;

      setMeta({
        name: data.name,
        description: data.description,
        minzoom: data.minzoom,
        maxzoom: data.maxzoom,
        center: center as [lng: number, lat: number, z: number],
        bounds: bounds as [x: number, y: number, z: number],
      });
    };

    fetchMVTMeta();
  }, [url]);

  const mergedAppearances: MVTAppearances | undefined = useMemo(
    () => ({
      ...appearances,
      marker: {
        ...DEFAULT_APPEARNACES.marker,
        ...(appearances?.marker ?? {}),
      },
      polyline: {
        ...DEFAULT_APPEARNACES.polyline,
        ...(appearances?.polyline ?? {}),
      },
      polygon: {
        ...DEFAULT_APPEARNACES.polygon,
        ...(appearances?.polygon ?? {}),
      },
      raster: {
        maximumLevel: meta?.maxzoom,
        hideIndicator: true,
      },
    }),
    [appearances, meta],
  );

  const data: Data = useMemo(
    () => ({
      type: "mvt",
      url,
      layers,
      jsonProperties: ["attributes", "dm_attributes"],
    }),
    [url, layers],
  );

  const handleOnLoad = useCallback(
    (layerId: string) => {
      if (!meta) return;
      if (visible) {
        sortedLayers?.forEach(layer => {
          if (layer?.layerId) {
            window.reearth?.layers?.bringToFront?.(layer.layerId);
          }
        });
      }
      onLoad?.(layerId, {
        lng: meta.center[0],
        lat: meta.center[1],
        height: 30000,
        pitch: -(Math.PI / 2),
        heading: 0,
        roll: 0,
      });
    },
    [meta, onLoad, sortedLayers, visible],
  );

  useLayer({
    data,
    visible,
    appearances: mergedAppearances,
    onLoad: handleOnLoad,
  });

  return null;
};
