import {
  ArcGISTiledElevationTerrainProvider,
  CesiumTerrainProvider,
  EllipsoidTerrainProvider,
  IonResource,
  TerrainProvider,
} from "cesium";
import { useEffect, useMemo } from "react";
import { Globe as CesiumGlobe } from "resium";

import type { ViewerProperty, TerrainProperty } from "../..";
import { AssetsCesiumProperty } from "../../../Map";
import { toColor } from "../common";

export type Props = {
  property?: ViewerProperty;
  cesiumIonAccessToken?: string;
  onTerrainProviderChange?: () => void;
};

export default function Globe({
  property,
  cesiumIonAccessToken,
  onTerrainProviderChange,
}: Props): JSX.Element | null {
  const terrainProperty = useMemo(
    (): TerrainProperty => ({
      ...property?.terrain,
    }),
    [property?.terrain],
  );

  const terrainProvider = useMemo((): Promise<TerrainProvider> | TerrainProvider | undefined => {
    const opts = {
      terrain: terrainProperty?.enabled,
      terrainType: terrainProperty?.type,
      normal: terrainProperty?.normal,
      ionAccessToken: property?.assets?.cesium?.terrain?.ionAccessToken || cesiumIonAccessToken,
      ionAsset: property?.assets?.cesium?.terrain?.ionAsset,
      ionUrl: property?.assets?.cesium?.terrain?.ionUrl,
    };
    const provider = opts.terrain ? terrainProviders[opts.terrainType || "cesium"] : undefined;
    return (typeof provider === "function" ? provider(opts) : provider) ?? defaultTerrainProvider;
  }, [
    terrainProperty?.enabled,
    terrainProperty?.type,
    terrainProperty?.normal,
    property?.assets?.cesium?.terrain?.ionAccessToken,
    property?.assets?.cesium?.terrain?.ionAsset,
    property?.assets?.cesium?.terrain?.ionUrl,
    cesiumIonAccessToken,
  ]);

  const baseColor = useMemo(
    () => toColor(property?.globe?.baseColor),
    [property?.globe?.baseColor],
  );

  useEffect(() => {
    onTerrainProviderChange?.();
  }, [terrainProvider, onTerrainProviderChange]);

  return (
    <CesiumGlobe
      baseColor={baseColor}
      enableLighting={!!property?.globe?.enableLighting}
      showGroundAtmosphere={property?.globe?.atmosphere?.enabled ?? true}
      atmosphereLightIntensity={property?.globe?.atmosphere?.lightIntensity}
      atmosphereSaturationShift={property?.globe?.atmosphere?.saturationShift}
      atmosphereHueShift={property?.globe?.atmosphere?.hueShift}
      atmosphereBrightnessShift={property?.globe?.atmosphere?.brightnessShift}
      terrainProvider={terrainProvider}
      depthTestAgainstTerrain={!!property?.globe?.depthTestAgainstTerrain}
    />
  );
}

const defaultTerrainProvider = new EllipsoidTerrainProvider();

const terrainProviders: {
  [k in NonNullable<TerrainProperty["type"]>]:
    | TerrainProvider
    | ((
        opts: Pick<TerrainProperty, "normal"> & AssetsCesiumProperty["terrain"],
      ) => Promise<TerrainProvider> | TerrainProvider | null);
} = {
  cesium: ({ ionAccessToken, normal }) =>
    CesiumTerrainProvider.fromUrl(
      IonResource.fromAssetId(1, {
        accessToken: ionAccessToken,
      }),
      {
        requestVertexNormals: normal,
        requestWaterMask: false,
      },
    ),
  arcgis: () =>
    ArcGISTiledElevationTerrainProvider.fromUrl(
      "https://elevation3d.arcgis.com/arcgis/rest/services/WorldElevation3D/Terrain3D/ImageServer",
    ),
  cesiumion: ({ ionAccessToken, ionAsset, ionUrl, normal }) =>
    ionAsset
      ? CesiumTerrainProvider.fromUrl(
          ionUrl ||
            IonResource.fromAssetId(parseInt(ionAsset, 10), {
              accessToken: ionAccessToken,
            }),
          {
            requestVertexNormals: normal,
          },
        )
      : null,
};
