import { SpatialIdSpaceData } from "../reearth/types/reearthPluginAPIv2/spatialId";

export const SPATIAL_ID_OBJECT = "SPATIAL_ID_OBJECT";

export type SpatialIdFeature = {
  id: string;
  data: SpatialIdSpaceData;
}

declare module "../../prototypes/screen-space-selection" {
  interface ScreenSpaceSelectionOverrides {
    [SPATIAL_ID_OBJECT]: string;
  }
}
