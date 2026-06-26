/** Correspond to https://github.com/takram-design-engineering/plateau-view/blob/main/libs/cesium/src/Environment.tsx */

import { FC, useEffect, useMemo, useRef } from "react";

import { useTerrainNormal, useTerrainUrl } from "../../states/environmentVariables";
import { AmbientOcclusion, Antialias, CameraPosition, Tile, TileLabels } from "../types";
import { isReEarthAPIv2 } from "../utils/reearth";

// nx = red
// ny = green
// nz = blue
// px = cyan
// py = magenta
// pz = yellow
const debugSphericalHarmonicCoefficients: [x: number, y: number, z: number][] = [
  [0.499745965003967, 0.499196201562881, 0.500154078006744], // L00, irradiance, pre-scaled base
  [0.265826553106308, -0.266099184751511, 0.265922993421555], // L1-1, irradiance, pre-scaled base
  [0.243236944079399, 0.266723394393921, -0.265380442142487], // L10, irradiance, pre-scaled base
  [-0.266895800828934, 0.265416264533997, 0.266921550035477], // L11, irradiance, pre-scaled base
  [0.000195000306121, -0.000644546060357, -0.000383183418307], // L2-2, irradiance, pre-scaled base
  [-0.000396036746679, -0.000622032093816, 0.000262127199676], // L2-1, irradiance, pre-scaled base
  [-0.000214280473301, 0.00004872302452, -0.000059724134189], // L20, irradiance, pre-scaled base
  [0.000107143961941, -0.000126510843984, -0.000425444566645], // L21, irradiance, pre-scaled base
  [-0.000069071611506, 0.000134039684781, -0.000119135256682], // L22, irradiance, pre-scaled base
];

export type EnvironmentProps = {
  backgroundColor?: string;
  globeBaseColor?: string;
  showGlobe?: boolean;
  enableGlobeLighting?: boolean;
  globeImageBasedLightingFactor?: number;
  terrainHeatmap?: boolean;
  lightColor?: string;
  lightIntensity?: number;
  shadowDarkness?: number;
  imageBasedLightingIntensity?: number;
  sphericalHarmonicCoefficients?: [x: number, y: number, z: number][];
  debugSphericalHarmonics?: boolean;
  showSun?: boolean;
  showMoon?: boolean;
  showSkyBox?: boolean;
  enableFog?: boolean;
  fogDensity?: number;
  showSkyAtmosphere?: boolean;
  showGroundAtmosphere?: boolean;
  atmosphereSaturationShift?: number;
  atmosphereBrightnessShift?: number;
  skyAtmosphereSaturationShift?: number;
  skyAtmosphereBrightnessShift?: number;
  groundAtmosphereSaturationShift?: number;
  groundAtmosphereBrightnessShift?: number;
};

export type ShadowProps = { enabled?: boolean; size?: 1024 | 2048 | 4096; softShadows?: boolean };

export type SceneProps = EnvironmentProps & {
  ambientOcclusion?: AmbientOcclusion;
  tiles?: Tile[];
  tileLabels?: TileLabels[];
  shadows?: ShadowProps;
  antialias?: Antialias;
  terrainHeatmapMaxHeight?: number;
  terrainHeatmapMinHeight?: number;
  terrainHeatmapLogarithmic?: boolean;
  initialCamera?: CameraPosition;
  enterUnderground?: boolean;
  hideUnderground?: boolean;
};

export const Scene: FC<SceneProps> = ({
  backgroundColor = "#000000",
  globeBaseColor = "#000000",
  // showGlobe = true,
  enableGlobeLighting = false,
  globeImageBasedLightingFactor = 0.3,
  terrainHeatmap = false,
  terrainHeatmapLogarithmic,
  terrainHeatmapMinHeight,
  terrainHeatmapMaxHeight,
  lightColor = "#ffffff",
  lightIntensity = 2,
  shadowDarkness = 0.3,
  imageBasedLightingIntensity = 1,
  sphericalHarmonicCoefficients,
  debugSphericalHarmonics = false,
  showSun = true,
  showMoon = false,
  showSkyBox = true,
  hideUnderground = false,
  enterUnderground = true,
  enableFog = true,
  fogDensity = 0.0002,
  showSkyAtmosphere = true,
  showGroundAtmosphere = true,
  atmosphereSaturationShift = 0,
  atmosphereBrightnessShift = 0,
  skyAtmosphereSaturationShift,
  skyAtmosphereBrightnessShift,
  groundAtmosphereSaturationShift,
  groundAtmosphereBrightnessShift,
  tiles,
  tileLabels,
  ambientOcclusion,
  shadows,
  antialias,
  initialCamera,
}) => {
  const [terrainUrl] = useTerrainUrl();
  const [terrainNormal] = useTerrainNormal();
  // CesiumTerrainProvider.fromUrl appends "/layer.json" to the given URL, so we
  // must pass the terrain base URL (e.g. ".../terrain"), not the layer.json URL
  // itself. Otherwise it requests ".../terrain/layer.json/layer.json" (404),
  // which makes Cesium retry the terrain load endlessly. Strip a trailing
  // "/layer.json" (and slashes) so either form configured in the widget works.
  const terrainBaseUrl = useMemo(
    () => terrainUrl?.replace(/\/+$/, "").replace(/\/layer\.json$/i, "") || undefined,
    [terrainUrl],
  );
  // The deps of this effect (tiles, tileLabels, shadows, initialCamera, ...) can be
  // recreated with a new identity on every parent render, re-running it dozens of times
  // per second even when nothing actually changed. Each overrideProperty call makes
  // Re:Earth rebuild the engine ViewerProperty, which re-creates the Cesium terrain
  // provider and causes an endless terrain reload loop. Gate on the serialized payload we
  // are about to send (NOT the inputs) so we only push when the payload truly changes —
  // this also ignores churn in inputs (e.g. initialCamera) that don't affect the payload.
  const lastPropertySigRef = useRef<string | undefined>(undefined);
  useEffect(() => {
    const pushIfChanged = (property: object, push: (p: never) => void) => {
      const sig = JSON.stringify(property);
      if (sig === lastPropertySigRef.current) return;
      lastPropertySigRef.current = sig;
      push(property as never);
    };

    if (isReEarthAPIv2(window?.reearth)) {
      const reearth = window.reearth;
      const property = {
        camera: {
          allowEnterGround: enterUnderground,
        },
        scene: {
          backgroundColor,
          light: {
            color: lightColor,
            intensity: debugSphericalHarmonics ? 0.5 : lightIntensity,
          },
          shadow: {
            darkness: globeImageBasedLightingFactor,
            enabled: shadows?.enabled,
            shadowMap: {
              darkness: shadowDarkness,
              size: shadows?.size,
              softShadows: shadows?.softShadows,
              maximumDistance: 10000,
            },
          },
          imageBasedLighting: {
            enabled: enableGlobeLighting,
            intensity: imageBasedLightingIntensity,
            sphericalHarmonicCoefficients: debugSphericalHarmonics
              ? debugSphericalHarmonicCoefficients
              : sphericalHarmonicCoefficients,
          },
        },
        sky: {
          skyBox: {
            show: showSkyBox,
          },
          fog: {
            enabled: enableFog,
            density: fogDensity,
          },
          sun: {
            show: showSkyBox && showSun,
          },
          moon: {
            show: showSkyBox && showMoon,
          },
          skyAtmosphere: {
            show: showSkyAtmosphere,
            saturationShift: skyAtmosphereSaturationShift ?? atmosphereSaturationShift,
            brightnessShift: skyAtmosphereBrightnessShift ?? atmosphereBrightnessShift,
          },
        },
        globe: {
          baseColor: globeBaseColor,
          enableLighting: enableGlobeLighting,
          atmosphere: {
            enabled: showGroundAtmosphere,
            brightnessShift: groundAtmosphereBrightnessShift ?? atmosphereBrightnessShift,
            saturationShift: groundAtmosphereSaturationShift ?? atmosphereSaturationShift,
          },
          depthTestAgainstTerrain: hideUnderground,
        },
        tiles: tiles?.map(t => ({
          id: t.id,
          type: t.tile_type,
          url: t.tile_url,
          opacity: t.tile_opacity,
          heatmap: t.heatmap,
          zoomLevel:
            t.tile_minLevel !== undefined && t.tile_maxLevel !== undefined
              ? [(t.tile_minLevel, t.tile_maxLevel)]
              : undefined,
        })),
        tileLabels,
        terrain: {
          enabled: true,
          type: "cesiumion",
          normal: terrainNormal ?? true,
          ...(terrainHeatmap
            ? {
                elevationHeatMap: {
                  type: "custom",
                  maxHeight: terrainHeatmapMaxHeight,
                  minHeight: terrainHeatmapMinHeight,
                  logarithmic: terrainHeatmapLogarithmic,
                },
              }
            : {}),
        },
        geoid: {
          server: {
            url: "https://api.plateauview.mlit.go.jp/citygml/geoid_height?lat=${lat}&lng=${lng}",
            geoidProperty: "geoid_height",
          },
        },
        render: {
          antialias,
          ambientOcclusion,
        },
        assets: {
          cesium: {
            terrain: {
              ionAccessToken:
                "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJiODVhMmQ5OS1hOWZjLTQ3YmYtODlmNi1lNWUwY2MwOGUxYTMiLCJpZCI6MTQ5ODk3LCJpYXQiOjE2ODc5MzQ3NDN9.OG0mc3i7ZxGwHQjlMv3TRjiOvKWpzxglxmJRaUIykTY",
              ionAsset: "4195264",
              ...(terrainBaseUrl ? { ionUrl: terrainBaseUrl } : {}),
            },
          },
        },
      };
      pushIfChanged(property, p => reearth?.viewer?.overrideProperty?.(p));
    } else {
      const reearth = window.reearth;
      const property = {
        default: {
          camera: initialCamera,
          bgcolor: backgroundColor,
          skybox: showSkyBox,
          allowEnterGround: enterUnderground,
        },
        atmosphere: {
          // Globe
          enable_lighting: enableGlobeLighting,
          globeImageBasedLighting: enableGlobeLighting,
          globeShadowDarkness: globeImageBasedLightingFactor,
          globeBaseColor,

          // Shadow
          shadowDarkness,
          shadows: shadows?.enabled,
          shadowResolution: shadows?.size,
          softShadow: shadows?.softShadows,
          shadowMaximumDistance: 10000,

          // Camera
          fog: enableFog,
          fog_density: fogDensity,

          // Sun
          enable_sun: showSkyBox && showSun,

          // Moon
          enableMoon: showSkyBox && showMoon,

          // Sky
          sky_atmosphere: showSkyAtmosphere,
          skyboxSurturationShift: skyAtmosphereSaturationShift ?? atmosphereSaturationShift,
          skyboxBrightnessShift: skyAtmosphereBrightnessShift ?? atmosphereBrightnessShift,

          // Ground
          ground_atmosphere: showGroundAtmosphere,
          surturation_shift: groundAtmosphereSaturationShift ?? atmosphereSaturationShift,
          brightness_shift: groundAtmosphereBrightnessShift ?? atmosphereBrightnessShift,
        },
        ambientOcclusion,
        light: {
          lightColor,
          lightIntensity: debugSphericalHarmonics ? 0.5 : lightIntensity,

          // Spherical harmonic
          sphericalHarmonicCoefficients: debugSphericalHarmonics
            ? debugSphericalHarmonicCoefficients
            : sphericalHarmonicCoefficients,
          imageBasedLightIntensity: imageBasedLightingIntensity,
        },
        tiles,
        tileLabels,
        terrain: {
          terrain: true,
          terrainType: "cesiumion",
          depthTestAgainstTerrain: hideUnderground,
          terrainCesiumIonAccessToken:
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJiODVhMmQ5OS1hOWZjLTQ3YmYtODlmNi1lNWUwY2MwOGUxYTMiLCJpZCI6MTQ5ODk3LCJpYXQiOjE2ODc5MzQ3NDN9.OG0mc3i7ZxGwHQjlMv3TRjiOvKWpzxglxmJRaUIykTY",
          terrainCesiumIonAsset: "4195264",
          ...(terrainBaseUrl ? { terrainCesiumIonUrl: terrainBaseUrl } : {}),
          terrainNormal: terrainNormal ?? true,
          ...(terrainHeatmap
            ? {
                heatmapType: "custom",
                heatmapMaxHeight: terrainHeatmapMaxHeight,
                heatmapMinHeight: terrainHeatmapMinHeight,
                heatmapLogarithmic: terrainHeatmapLogarithmic,
              }
            : {}),
        },
        render: {
          antialias,
        },
      };
      pushIfChanged(property, p => reearth?.scene?.overrideProperty(p));
    }
  }, [
    antialias,
    ambientOcclusion,
    atmosphereBrightnessShift,
    atmosphereSaturationShift,
    backgroundColor,
    debugSphericalHarmonics,
    enableFog,
    enableGlobeLighting,
    fogDensity,
    globeImageBasedLightingFactor,
    groundAtmosphereBrightnessShift,
    groundAtmosphereSaturationShift,
    imageBasedLightingIntensity,
    lightColor,
    lightIntensity,
    shadowDarkness,
    showGroundAtmosphere,
    showSkyAtmosphere,
    showSkyBox,
    showSun,
    showMoon,
    sphericalHarmonicCoefficients,
    tiles,
    tileLabels,
    shadows,
    globeBaseColor,
    skyAtmosphereBrightnessShift,
    skyAtmosphereSaturationShift,
    terrainHeatmap,
    terrainHeatmapLogarithmic,
    terrainHeatmapMaxHeight,
    terrainHeatmapMinHeight,
    initialCamera,
    enterUnderground,
    hideUnderground,
    terrainBaseUrl,
    terrainNormal,
  ]);

  useEffect(() => {
    if (initialCamera) {
      window.reearth?.camera?.flyTo(initialCamera, { duration: 0 });
    }
  }, [initialCamera]);

  return null;
};
