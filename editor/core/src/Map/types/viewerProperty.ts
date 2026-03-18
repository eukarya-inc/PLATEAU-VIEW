/*
  * ViewerProperty Interface
  * ------------------------
  * The ViewerProperty interface defines a collection of sub-properties that represent different aspects of the viewer component.
  * Each sub-property encapsulates specific settings and configurations related to the viewer's behavior and appearance.
  
  * GlobeProperty: Settings related to the globe visualization, such as base color, lighting, and atmosphere.
  * TerrainProperty: Configuration options for terrain visualization, including terrain type, height map, and normal mapping.
  * SceneProperty: Scene-specific settings, such as background color, scene mode, vertical exaggeration, and lighting.
  * TilesProperty: Properties for managing tile layers, including URLs, opacity, zoom levels, and heatmap settings.
  * TileLabelProperty: Properties for managing tile labels, such as label type and style.
  * SkyProperty: Settings for sky visualization, including skybox, sun, moon, fog, and sky atmosphere.
  * CameraProperty: Camera-related settings, such as camera position, limiter, and ground entry.
  * RenderProperty: Rendering settings, such as antialiasing and ambient occlusion.
  * IndicatorProperty: Properties for customizing the viewer's indicator, such as type and image.
  * AssetsProperty: Asset-related settings, such as access tokens and URLs for external assets.
  * DebugProperty: Debugging options, such as wireframe display and FPS counter.
  
  * Guidelines for Adding New Properties
  * ------------------------------------
  * When adding new properties to the ViewerProperty interface, follow these steps to ensure consistency and maintainability:
  * 1. Determine the Appropriate Category: Identify the aspect of the viewer your new property will affect and choose the most relevant existing sub-property type (e.g., GlobeProperty, TerrainProperty, SceneProperty, etc.). If your property does not fit into an existing category, consider creating a new sub-property type.
  * 2. Define the New Property: Add your new property to the chosen sub-property type. Ensure that the property name is descriptive and the type is correctly specified. If your property requires complex structures, define new types as needed.
  * 3. Maintain Optional Properties: To keep the interface flexible, add new properties as optional (?) unless they are strictly required.
  * 4. Follow Naming Rules:
  * - Use CamelCase for property names.
  * - Keep property names descriptive but concise.
  * - Maintain consistency with existing naming patterns.
  * - Avoid abbreviations unless widely understood and standard.
  * 5. Document the Property: Add comments or documentation to describe the purpose and usage of the new property. This will help other developers understand its function and how to use it.
  * 6. Update Usage Examples: If applicable, update any example configurations or documentation to include the new property, demonstrating how it should be used.
*/

import type { LUT, Camera } from "../../mantle";

export type SceneMode = "3d" | "2d" | "columbus";

export type ViewerProperty = {
  globe?: GlobeProperty;
  geoid?: GeoidProperty;
  terrain?: TerrainProperty;
  scene?: SceneProperty;
  tiles?: TileProperty[];
  tileLabels?: TileLabelProperty[];
  sky?: SkyProperty;
  camera?: CameraProperty;
  render?: RenderPeropty;
  assets?: AssetsProperty; // anything related to specific assets and its access tokens
  debug?: DebugProperty;
  indicator?: IndicatorProperty; // consider remove this if not needed in the future
};

export type GlobeProperty = {
  baseColor?: string;
  enableLighting?: boolean;
  atmosphere?: GlobeAtmosphereProperty;
  depthTestAgainstTerrain?: boolean;
};

export type GeoidProperty = {
  server: {
    url: string;
    geoidProperty: string;
  };
};

export type GlobeAtmosphereProperty = {
  enabled?: boolean;
  lightIntensity?: number;
  brightnessShift?: number;
  hueShift?: number;
  saturationShift?: number;
};

export type TerrainProperty = {
  enabled?: boolean;
  type?: "cesium" | "arcgis" | "cesiumion";
  url?: string;
  normal?: boolean;
  elevationHeatMap?: ElevationHeatMapProperty;
};

export type ElevationHeatMapProperty = {
  type?: "custom";
  colorLUT?: LUT;
  minHeight?: number;
  maxHeight?: number;
  logarithmic?: boolean;
};

export type SceneProperty = {
  backgroundColor?: string;
  mode?: SceneMode;
  verticalExaggeration?: number;
  verticalExaggerationRelativeHeight?: number;
  vr?: boolean;
  light?: LightProperty;
  shadow?: ShadowProperty;
  imageBasedLighting?: ImageBasedLighting;
};

export type LightProperty = {
  type?: "sunLight" | "directionalLight";
  direction?: [x: number, y: number, z: number];
  color?: string;
  intensity?: number;
};

export type ShadowProperty = {
  enabled?: boolean;
  darkness?: number;
  shadowMap?: ShadowMapProperty;
};

export type ShadowMapProperty = {
  size?: 1024 | 2048 | 4096;
  softShadows?: boolean;
  darkness?: number;
  maximumDistance?: number;
};

export type ImageBasedLighting = {
  enabled?: boolean;
  intensity?: number;
  specularEnvironmentMaps?: string;
  sphericalHarmonicCoefficients?: [number, number, number][];
};

export type TileProperty = {
  id: string;
  type?: string;
  url?: string;
  opacity?: number;
  zoomLevel?: number[];
  zoomLevelForURL?: number[];
  heatmap?: boolean;
};

export type TileLabelProperty = {
  id: string;
  labelType: "japan_gsi_optimal_bvmap";
  style: Record<string, any>;
  near?: number;
  far?: number;
};

export type SkyProperty = {
  skyBox?: SkyBoxProperty;
  sun?: SunProperty;
  moon?: MoonProperty;
  fog?: FogProperty;
  skyAtmosphere?: SkyAtmosphereProperty;
};

export type SkyBoxProperty = {
  show?: boolean;
};

export type SunProperty = {
  show?: boolean;
};

export type MoonProperty = {
  show?: boolean;
};

export type FogProperty = {
  enabled?: boolean;
  density?: number;
};

export type SkyAtmosphereProperty = {
  show?: boolean;
  lightIntensity?: number;
  saturationShift?: number;
  brightnessShift?: number;
};

export type CameraProperty = {
  allowEnterGround?: boolean;
  limiter?: CameraLimiterProperty;
};

export type CameraLimiterProperty = {
  enabled?: boolean;
  targetArea?: Camera;
  targetWidth?: number;
  targetLength?: number;
  showHelper?: boolean;
};

export type RenderPeropty = {
  antialias?: "low" | "medium" | "high" | "extreme";
  ambientOcclusion?: AmbientOcclusionProperty;
};

export type AmbientOcclusionProperty = {
  enabled?: boolean;
  quality?: "low" | "medium" | "high" | "extreme";
  intensity?: number;
  ambientOcclusionOnly?: boolean;
};

export type IndicatorProperty = {
  type?: "default" | "crosshair" | "custom";
  image?: string;
  imageScale?: number;
};

export type AssetsProperty = {
  cesium?: AssetsCesiumProperty;
};

export type AssetsCesiumProperty = {
  terrain?: {
    ionAccessToken?: string;
    ionAsset?: string;
    ionUrl?: string;
  };
};

export type DebugProperty = {
  showGlobeWireframe?: boolean;
  showFramesPerSecond?: boolean;
};
