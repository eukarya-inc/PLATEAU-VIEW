import { ViewerProperty } from "@reearth/core";

// Ref: https://github.com/eukarya-inc/PLATEAU-VIEW-3.0/blob/cfcb4b6a444fc9695b4089c8224016d9650cf2b7/extension/src/shared/reearth/scene/Scene.tsx#L13
// eslint-disable-next-line @typescript-eslint/no-unused-vars
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

// Ref: https://github.com/eukarya-inc/PLATEAU-VIEW-3.0/blob/cfcb4b6a444fc9695b4089c8224016d9650cf2b7/extension/src/prototypes/view/environments/SatelliteEnvironment.tsx#L10
const sphericalHarmonicCoefficients: [x: number, y: number, z: number][] = [
  [1.221931219100952, 1.266084671020508, 1.019550442695618],
  [0.800345599651337, 0.841745376586914, 0.723761379718781],
  [0.912390112876892, 0.922998011112213, 0.649103164672852],
  [-0.843475937843323, -0.853787302970886, -0.601324439048767],
  [-0.495116978883743, -0.5034259557724, -0.360104471445084],
  [0.497776478528976, 0.507052302360535, 0.364346027374268],
  [0.082192525267601, 0.082608506083488, 0.056836795061827],
  [-0.925247848033905, -0.940086245536804, -0.678709805011749],
  [0.114833705127239, 0.114355310797691, 0.067587599158287],
];

const TILE_LABELS: ViewerProperty["tileLabels"] = [
  {
    id: `label`,
    labelType: "japan_gsi_optimal_bvmap",
    near: 0,
    far: 3000,
    style: {
      municipalities: {
        fillColor: "#000000",
        outlineColor: "rgba(255, 255, 255, 0.8)",
      },
      towns: {
        fillColor: "rgba(0, 0, 0, 0.6)",
      },
      topography: {
        fillColor: "rgba(0, 0, 0, 0.6)",
      },
    },
  },
];

export const DEFAULT_VIEWER_PROPERTY: ViewerProperty = {
  tileLabels: TILE_LABELS,
  globe: {
    enableLighting: true,
  },
  scene: {
    imageBasedLighting: {
      enabled: true,
      sphericalHarmonicCoefficients,
    },
  },
  terrain: {
    enabled: true,
    normal: true,
  },
  geoid: {
    server: {
      url: "https://api-vt.geolonia.com/api/altitude?lat=${lat}&lng=${lng}",
      geoidProperty: "geoid",
    },
  },
};
