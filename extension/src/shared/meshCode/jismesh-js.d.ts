declare module "jismesh-js" {
  export function toMeshCode(latitude: number, longitude: number, zoomLevel: number): string;
  export function toMeshPoint(
    meshCode: string,
    index1: number,
    index2: number,
  ): [latitude: number, longitude: number];
  export function toMeshLevel(meshCode: string): number;
}
