export function isMeshCodeType(value: string): value is MeshCodeType {
  return value === "2x" || value === "3x";
}

export const MESH_CODE_OBJECT = "MESH_CODE_OBJECT";

export type MeshCodeType = "2x" | "3x";

export type MeshCodeFeature = {
  id: string;
  meshCode: string;
};

declare module "../../prototypes/screen-space-selection" {
  interface ScreenSpaceSelectionOverrides {
    [MESH_CODE_OBJECT]: string;
  }
}
