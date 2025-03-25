export type InteractionModeType = "default" | "move" | "selection" | "sketch" | "spatialId";

export type InteractionMode = {
  override?: (mode: InteractionModeType) => void;
  mode: InteractionModeType;
};
