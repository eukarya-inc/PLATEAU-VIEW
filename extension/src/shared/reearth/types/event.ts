import { CameraPosition } from "./camera";
import { SpatialIdSpaceData } from "./reearthPluginAPIv2/spatialId";
import { PickedFeature } from "./scene";
import { SketchFeature } from "./sketch";

export type MouseEvent = {
  x?: number;
  y?: number;
  lat?: number;
  lng?: number;
  height?: number;
  layerId?: string;
  delta?: number;
};

export type LatLngHeight = {
  lat: number;
  lng: number;
  height: number;
};

export type LayerEditEvent = {
  layerId: string | undefined;
  scale?: {
    width: number;
    length: number;
    height: number;
    location: LatLngHeight;
  };
  rotate?: {
    heading: number;
    pitch: number;
    roll: number;
  };
};

export type LayerVisibilityEvent = {
  layerId: string | undefined;
};

export type LayerLoadEvent = {
  layerId: string | undefined;
};

export type LayerSelectWithRect = MouseEvent & { pressedKey?: "shift" };
export type LayerSelectWithRectStart = LayerSelectWithRect;
export type LayerSelectWithRectMove = LayerSelectWithRect & {
  startX?: number;
  startY?: number;
  width?: number;
  height?: number;
};
export type LayerSelectWithRectEnd = LayerSelectWithRect & {
  features: PickedFeature[] | undefined;
  isClick?: boolean;
};

export type ReearthEventType = {
  update: [];
  close: [];
  cameramove: [camera: CameraPosition];
  //   layeredit: [e: LayerEditEvent];
  select: [layerId: string | undefined, featureId: string | undefined];
  message: [message: any];
  click: [props: MouseEvent];
  doubleclick: [props: MouseEvent];
  mousedown: [props: MouseEvent];
  mouseup: [props: MouseEvent];
  rightclick: [props: MouseEvent];
  rightdown: [props: MouseEvent];
  rightup: [props: MouseEvent];
  middleclick: [props: MouseEvent];
  middledown: [props: MouseEvent];
  middleup: [props: MouseEvent];
  mousemove: [props: MouseEvent];
  mouseenter: [props: MouseEvent];
  mouseleave: [props: MouseEvent];
  wheel: [props: MouseEvent];
  tick: [props: Date];
  //   resize: [props: ViewportSize];
  modalclose: [];
  popupclose: [];
  //   pluginmessage: [props: PluginMessage];
  sketchfeaturecreated: [
    props: {
      layerId: string;
      featureId: string;
      feature?: SketchFeature;
    },
  ];
  spatialidspacepick: [props: SpatialIdSpaceData];
  layerVisibility: [e: LayerVisibilityEvent];
  layerload: [e: LayerLoadEvent];
  layerSelectWithRectStart: [e: LayerSelectWithRectStart];
  layerSelectWithRectMove: [e: LayerSelectWithRectMove];
  layerSelectWithRectEnd: [e: LayerSelectWithRectEnd];
};
