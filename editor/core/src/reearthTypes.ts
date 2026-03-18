// TODO: Remove this file after the migration is done
// TODO: Move infobox types to where layer type locates
import type { Spacing } from "./mantle";

export type Infobox<BP = any> = {
  featureId?: string;
  property?: InfoboxProperty;
  blocks?: InfoboxBlock<BP>[];
};

export type InfoboxProperty = {
  default?: {
    enabled?: PropertyItem<boolean>;
    position?: PropertyItem<"right" | "left">;
    padding?: PropertyItem<Spacing>;
    gap?: PropertyItem<number>;
  };
  // for compat
  defaultContent?: "description" | "attributes";
};

export type InfoboxBlock<P = any> = {
  id: string;
  name?: string;
  pluginId?: string;
  extensionId?: string;
  property?: P;
  propertyId?: string;
};

export type PropertyItem<T> = {
  type?: string;
  ui?: string;
  title?: string;
  description?: string;
  value?: T;
  min?: number;
  max?: number;
  choices?: { [key: string]: string }[];
};

// @reearth/types
export type Args<F> = F extends (a: any, ...b: infer A) => any ? A : never;
export type Args2<F> = F extends (a: any, b: any, ...c: infer A) => any ? A : never;
export type Args3<F> = F extends (a: any, b: any, c: any, ...d: infer A) => any ? A : never;

// @reearth/services/gql/__gen__/graphql.ts
export enum ValueType {
  Array = "ARRAY",
  Bool = "BOOL",
  Camera = "CAMERA",
  Coordinates = "COORDINATES",
  Latlng = "LATLNG",
  Latlngheight = "LATLNGHEIGHT",
  Number = "NUMBER",
  Polygon = "POLYGON",
  Rect = "RECT",
  Ref = "REF",
  Spacing = "SPACING",
  String = "STRING",
  Timeline = "TIMELINE",
  Typography = "TYPOGRAPHY",
  Url = "URL",
}
