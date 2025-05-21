import {
  Color,
  ImageryProvider,
  TextureMagnificationFilter,
  TextureMinificationFilter,
} from "cesium";
import { isEqual } from "lodash-es";
import { useCallback, useMemo, useRef, useLayoutEffect, useState, useEffect } from "react";
import { ImageryLayer } from "resium";

import { tiles as tilePresets } from "./presets";

export type ImageryLayerData = {
  id: string;
  provider: ImageryProvider;
  min?: number;
  max?: number;
  opacity?: number;
};

export type Tile = {
  id: string;
  url?: string;
  type?: string;
  opacity?: number;
  zoomLevel?: number[];
  zoomLevelForURL?: number[];
  heatmap?: boolean;
};

export type Props = {
  tiles?: Tile[];
  cesiumIonAccessToken?: string;
  onTilesChange?: () => void;
};

export default function ImageryLayers({ tiles, cesiumIonAccessToken, onTilesChange }: Props) {
  const { providers, updated } = useImageryProviders({
    tiles,
    cesiumIonAccessToken,
    presets: tilePresets,
  });

  // force rerendering all layers when any provider is updated
  // since Resium does not sort layers according to ImageryLayer component order
  const [counter, setCounter] = useState(0);
  useLayoutEffect(() => {
    if (updated) setCounter(c => c + 1);
  }, [providers, updated]);

  useEffect(() => {
    onTilesChange?.();
  }, [tiles, counter, onTilesChange]);

  return (
    <>
      {tiles
        ?.map(({ id, ...tile }) => ({
          ...tile,
          id,
          provider: providers[id]?.[2],
        }))
        .map(({ id, opacity, zoomLevel, provider, heatmap }, i) =>
          provider ? (
            <ImageryLayer
              key={`${id}_${i}_${counter}`}
              imageryProvider={provider}
              minimumTerrainLevel={zoomLevel?.[0]}
              maximumTerrainLevel={zoomLevel?.[1]}
              alpha={opacity}
              index={i}
              colorToAlpha={heatmap ? Color.WHITE : undefined}
              colorToAlphaThreshold={heatmap ? 1 : undefined}
              magnificationFilter={heatmap ? TextureMagnificationFilter.LINEAR : undefined}
              minificationFilter={heatmap ? TextureMinificationFilter.NEAREST : undefined}
            />
          ) : null,
        )}
    </>
  );
}

type Providers = { [id: string]: [string | undefined, string | undefined, ImageryProvider] };

export function useImageryProviders({
  tiles = [],
  cesiumIonAccessToken,
  presets,
}: {
  tiles?: Tile[];
  cesiumIonAccessToken?: string;
  presets: {
    [key: string]: (opts?: {
      url?: string;
      cesiumIonAccessToken?: string;
      heatmap?: boolean;
      zoomLevel?: number[];
    }) => Promise<ImageryProvider> | ImageryProvider | null;
  };
}): { providers: Providers; updated: boolean } {
  const newTile = useCallback(
    (t: Tile, ciat?: string) =>
      presets[t.type || "default"]({
        url: t.url,
        cesiumIonAccessToken: ciat,
        heatmap: t.heatmap,
        zoomLevel: t.zoomLevelForURL,
      }),
    [presets],
  );

  const prevCesiumIonAccessToken = useRef(cesiumIonAccessToken);
  const tileKeys = tiles.map(t => t.id).join(",");
  const prevTileKeys = useRef(tileKeys);
  const prevProviders = useRef<Providers>({});
  const zoomLevels = useMemo(() => tiles.map(t => {
      if (t.id && t.zoomLevel) return { [t.id]: t.zoomLevel };
      return
  }),
  [tiles]);
  const prevZoomLevels = useRef(zoomLevels);

  // Manage TileProviders so that TileProvider does not need to be recreated each time tiles are updated.
  const { providers, updated } = useMemo(() => {
    const isCesiumAccessTokenUpdated = prevCesiumIonAccessToken.current !== cesiumIonAccessToken;
    const prevProvidersKeys = Object.keys(prevProviders.current);
    const added = tiles.map(t => t.id).filter(t => t && !prevProvidersKeys.includes(t));

    const rawProviders = [
      ...Object.entries(prevProviders.current),
      ...added.map(a => [a, undefined] as const),
    ].map(([k, v]) => ({
      key: k,
      added: added.includes(k),
      prevType: v?.[0],
      prevUrl: v?.[1],
      prevProvider: v?.[2],
      tile: tiles.find(t => t.id === k),
    }));

    const providers = Object.fromEntries(
      rawProviders
        .map(
          ({
            key,
            added,
            prevType,
            prevUrl,
            prevProvider,
            tile,
          }):
            | [
                string,
                [
                  string | undefined,
                  string | undefined,
                  Promise<ImageryProvider> | ImageryProvider | null | undefined,
                ],
              ]
            | null =>
            !tile
              ? null
              : [
                  key,
                  added ||
                  prevType !== tile.type ||
                  prevUrl !== tile.url ||
                  (isCesiumAccessTokenUpdated && (!tile.type || tile.type === "default"))
                    ? [tile.type, tile.url, newTile(tile, cesiumIonAccessToken)]
                    : [prevType, prevUrl, prevProvider],
                ],
        )
        .filter(
          (e): e is [string, [string | undefined, string | undefined, ImageryProvider]] =>
            !!e?.[1][2],
        ),
    );

    const updated =
      !!added.length ||
      !!isCesiumAccessTokenUpdated ||
      !isEqual(prevTileKeys.current, tileKeys) ||
      !isEqual(prevZoomLevels.current, zoomLevels) ||
      rawProviders.some(p => p.tile && (p.prevType !== p.tile.type || p.prevUrl !== p.tile.url));

    prevTileKeys.current = tileKeys;
    prevZoomLevels.current = zoomLevels;
    prevCesiumIonAccessToken.current = cesiumIonAccessToken;

    return { providers, updated };
  }, [cesiumIonAccessToken, tiles, tileKeys, newTile, zoomLevels]);

  prevProviders.current = providers;
  return { providers, updated };
}
