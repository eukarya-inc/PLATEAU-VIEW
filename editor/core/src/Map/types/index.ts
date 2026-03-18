import { Cartesian3 } from "cesium";
import { type LineString, type MultiPolygon, type Polygon, type Point } from "geojson";
import type {
  ForwardRefExoticComponent,
  PropsWithoutRef,
  RefAttributes,
  ReactNode,
  CSSProperties,
  RefObject,
  MutableRefObject,
} from "react";

import { PickedFeature } from "../../engines/Cesium/pickMany";
import { CursorType } from "../../engines/Cesium/types";
import type {
  LatLngHeight,
  Camera,
  Rect,
  LatLng,
  DataType,
  SelectedFeatureInfo,
  Feature,
  ComputedFeature,
  CameraPosition,
} from "../../mantle";
import type {
  CameraOptions,
  FlyTo,
  FlyToDestination,
  LookAtDestination,
  Position2d,
  Position3d,
  ScreenSpaceCameraControllerOptions,
} from "../../types";
import type {
  FeatureComponentType,
  ClusterComponentType,
  LayerSelectionReason,
  Ref as LayersRef,
} from "../Layers";
import {
  SketchType,
  SketchOptions,
  SketchComponentType,
  SketchEditFeatureChangeCb,
  SketchEditingFeature,
} from "../Sketch/types";
import type { TimelineManagerRef } from "../useTimelineManager";

import type { SceneMode, ViewerProperty } from "./viewerProperty";

export type {
  FeatureComponentProps,
  FeatureComponentType,
  ClusterComponentType,
  ClusterComponentProps,
  ClusterProperty,
  Ref as LayersRef,
  LayerSelectionReason,
} from "../Layers";
export type {
  Atom,
  DataType,
  DataRange,
  Feature,
  Layer,
  LayerSimple,
  ComputedFeature,
  ComputedLayer,
  Geometry,
  AppearanceTypes,
  Camera,
  Typography,
  LatLng,
  Rect,
  LatLngHeight,
  ValueTypes,
  ValueType,
} from "../../mantle";
export * from "./event";
export * from "./viewerProperty";

export type EngineRef = {
  name: string;
  requestRender: () => void;
  getViewport: () => Rect | undefined;
  getCamera: () => Camera | undefined;
  getCameraFovInfo: (options: { withTerrain?: boolean; calcViewSize?: boolean }) =>
    | {
        center?: LatLngHeight;
        viewSize?: number;
      }
    | undefined;
  getLocationFromScreen: (x: number, y: number, withTerrain?: boolean) => LatLngHeight | undefined;
  sampleTerrainHeight: (lng: number, lat: number) => Promise<number | undefined>;
  computeGlobeHeight: (lng: number, lat: number, height?: number) => number | undefined;
  getGlobeHeight: () => number | undefined;
  toXYZ: (
    lng: number,
    lat: number,
    height: number,
    options?: { useGlobeEllipsoid?: boolean },
  ) => [x: number, y: number, z: number] | undefined;
  toLngLatHeight: (
    x: number,
    y: number,
    z: number,
    options?: { useGlobeEllipsoid?: boolean },
  ) => [lng: number, lat: number, height: number] | undefined;
  convertScreenToPositionOffset: (
    rawPosition: [x: number, y: number, z: number],
    screenOffset: [x: number, y: number],
  ) => [x: number, y: number, z: number] | undefined;
  isPositionVisible: (position: [x: number, y: number, z: number]) => boolean;
  setView: (camera: CameraPosition) => void;
  toWindowPosition: (
    position: [x: number, y: number, z: number],
  ) => [x: number, y: number] | undefined;
  getExtrudedHeight: (
    position: [x: number, y: number, z: number],
    windowPosition: [x: number, y: number],
    allowNegative?: boolean,
  ) => number | undefined;
  getExtrudedPoint: (
    position: [x: number, y: number, z: number],
    extrutedHeight: number,
  ) => Position3d | undefined;
  getSurfaceDistance: (point1: Cartesian3, point2: Cartesian3) => number | undefined;
  equalsEpsilon2d: (
    point1: Position2d,
    point2: Position2d,
    relativeEpsilon: number | undefined,
    absoluteEpsilon: number | undefined,
  ) => boolean;
  equalsEpsilon3d: (
    point1: Position3d,
    point2: Position3d,
    relativeEpsilon: number | undefined,
    absoluteEpsilon: number | undefined,
  ) => boolean;
  createGeometry: ({
    type,
    controlPoints,
  }: {
    type: SketchType;
    controlPoints: Position3d[];
  }) => LineString | Polygon | MultiPolygon | Point | undefined;
  setCursor: (cursor: CursorType) => void;
  flyTo: FlyTo;
  flyToBBox: (
    bbox: [number, number, number, number],
    options?: CameraOptions & {
      heading?: number;
      pitch?: number;
      range?: number;
    },
  ) => void;
  rotateOnCenter: (radian: number) => void;
  overrideScreenSpaceController: (options: ScreenSpaceCameraControllerOptions) => void;
  lookAt: (destination: LookAtDestination, options?: CameraOptions) => void;
  lookAtLayer: (layerId: string) => void;
  zoomIn: (amount: number, options?: CameraOptions) => void;
  zoomOut: (amount: number, options?: CameraOptions) => void;
  orbit: (radians: number) => void;
  rotateRight: (radians: number) => void;
  changeSceneMode: (sceneMode: SceneMode | undefined, duration?: number) => void;
  getClock: () => Clock | undefined;
  captureScreen: (type?: string, encoderOptions?: number) => string | undefined;
  enableScreenSpaceCameraController: (enabled: boolean) => void;
  lookHorizontal: (amount: number) => void;
  lookVertical: (amount: number) => void;
  moveForward: (amount: number) => void;
  moveBackward: (amount: number) => void;
  moveUp: (amount: number) => void;
  moveDown: (amount: number) => void;
  moveLeft: (amount: number) => void;
  moveRight: (amount: number) => void;
  moveOverTerrain: (offset?: number) => void;
  flyToGround: (destination: FlyToDestination, options?: CameraOptions, offset?: number) => void;
  mouseEventCallbacks: MouseEventCallbacks;
  pause: () => void;
  play: () => void;
  changeSpeed: (speed: number) => void;
  changeStart: (start: Date) => void;
  changeStop: (stop: Date) => void;
  changeTime: (time: Date) => void;
  tick: () => Date | void;
  inViewport: (location?: LatLng) => boolean;
  onTick: TickEvent;
  tickEventCallback?: RefObject<TickEventCallback[]>;
  removeTickEventListener: TickEvent;
  findFeatureById: (layerId: string, featureId: string) => Feature | undefined;
  bringToFront: (layerId: string) => void;
  sendToBack: (layerId: string) => void;
  findFeaturesByIds: (layerId: string, featureId: string[]) => Feature[] | undefined;
  findComputedFeatureById: (layerId: string, featureId: string) => ComputedFeature | undefined;
  findComputedFeaturesByIds: (
    layerId: string,
    featureId: string[],
  ) => ComputedFeature[] | undefined;
  selectFeatures: (layerId: string, featureId: string[]) => void;
  unselectFeatures: (layerId: string, featureId: string[]) => void;
  pickManyFromViewport: (
    windowPosition: [x: number, y: number],
    windowWidth: number,
    windowHeight: number,
    // TODO: Get condition as expression for plugin
    condition?: (f: PickedFeature) => boolean,
  ) => PickedFeature[] | undefined;
  calcRectangleControlPoint: (
    p1: Position3d,
    p2: Position3d,
    p3: Position3d,
  ) => [p1: Position3d, p2: Position3d, p3: Position3d];
  getCredits: () => Credit[] | undefined;
} & MouseEventHandles;

export type EngineProps = {
  className?: string;
  style?: CSSProperties;
  isEditable?: boolean;
  isBuilt?: boolean;
  property?: ViewerProperty;
  time?: string | Date;
  camera?: Camera;
  cameraForceHorizontalRoll?: boolean;
  small?: boolean;
  children?: ReactNode;
  ready?: boolean;
  selectedLayerId?: {
    layerId?: string;
    featureId?: string;
  };
  featureFlags: number;
  layerSelectionReason?: LayerSelectionReason;
  isLayerDraggable?: boolean;
  isLayerDragging?: boolean;
  shouldRender?: boolean;
  meta?: Record<string, unknown>;
  displayCredits?: boolean;
  layersRef?: RefObject<LayersRef>;
  requestingRenderMode?: MutableRefObject<RequestingRenderMode>;
  timelineManagerRef?: TimelineManagerRef;
  onLayerSelect?: (
    layerId: string | undefined,
    featureId?: string,
    options?: LayerSelectionReason,
    info?: SelectedFeatureInfo,
  ) => void;
  onCameraChange?: (camera: Camera) => void;
  onLayerDrag?: (layerId: string, featureId: string | undefined, position: LatLng) => void;
  onLayerDrop?: (
    layerId: string,
    featureId: string | undefined,
    position: LatLng | undefined,
  ) => void;
  onLayerEdit?: (e: LayerEditEvent) => void;
  onMount?: () => void;
  onLayerVisibility?: (e: LayerVisibilityEvent) => void;
  onLayerLoad?: (e: LayerLoadEvent) => void;
  onLayerSelectWithRectStart?: (e: LayerSelectWithRectStart) => void;
  onLayerSelectWithRectMove?: (e: LayerSelectWithRectMove) => void;
  onLayerSelectWithRectEnd?: (e: LayerSelectWithRectEnd) => void;
  onCreditsUpdate?: (credits: Credit[]) => void;
};

export type LayerEditEvent = {
  layerId: string | undefined;
  scale?: {
    width: number;
    length: number;
    height: number;
    location: LatLngHeight;
  };
  rotate?: { heading: number; pitch: number; roll: number };
};

export type LayerSelectWithRect = MouseEventProps & { pressedKey?: "shift" };
export type LayerSelectWithRectStart = LayerSelectWithRect;
export type LayerSelectWithRectMove = LayerSelectWithRect & {
  startX?: number;
  startY?: number;
  width?: number;
  height?: number;
};
export type LayerSelectWithRectEnd = LayerSelectWithRect & {
  features: PickedFeature[] | undefined;
  isClick: boolean;
};

export type LayerVisibilityEvent = {
  layerId: string | undefined;
};

export type LayerLoadEvent = {
  layerId: string | undefined;
};

export type Clock = {
  current?: Date;
  start?: Date;
  stop?: Date;
  speed?: number;
  playing?: boolean;
};

export type MouseEventProps = {
  x?: number;
  y?: number;
  lat?: number;
  lng?: number;
  height?: number;
  layerId?: string;
  delta?: number;
};

export type MouseEventCallback = (props: MouseEventProps) => void;
export type MouseWheelEventCallback = (props: MouseEventProps) => void;
export type MouseEventTypes =
  | "click"
  | "doubleClick"
  | "mouseDown"
  | "mouseUp"
  | "rightClick"
  | "rightDown"
  | "rightUp"
  | "middleClick"
  | "middleDown"
  | "middleUp"
  | "mouseMove"
  | "mouseEnter"
  | "mouseLeave"
  | "wheel";

export type MouseEvents = {
  [key in MouseEventTypes]: MouseEventCallback;
} & {
  wheel: MouseWheelEventCallback;
};

export type MouseEventHandles = {
  onClick: (fn: MouseEvents["click"]) => void;
  onDoubleClick: (fn: MouseEvents["doubleClick"]) => void;
  onMouseDown: (fn: MouseEvents["mouseDown"]) => void;
  onMouseUp: (fn: MouseEvents["mouseUp"]) => void;
  onRightClick: (fn: MouseEvents["rightClick"]) => void;
  onRightDown: (fn: MouseEvents["rightDown"]) => void;
  onRightUp: (fn: MouseEvents["rightUp"]) => void;
  onMiddleClick: (fn: MouseEvents["middleClick"]) => void;
  onMiddleDown: (fn: MouseEvents["middleDown"]) => void;
  onMiddleUp: (fn: MouseEvents["middleUp"]) => void;
  onMouseMove: (fn: MouseEvents["mouseMove"]) => void;
  onMouseEnter: (fn: MouseEvents["mouseEnter"]) => void;
  onMouseLeave: (fn: MouseEvents["mouseLeave"]) => void;
  onWheel: (fn: MouseEvents["wheel"]) => void;
};

export type MouseEventCallbacks = { [key in keyof MouseEvents]: MouseEvents[key][] };

export type TickEvent = (cb: TickEventCallback) => void;
export type TickEventCallback = (current: Date, clock: { start: Date; stop: Date }) => void;

export type EngineComponent = ForwardRefExoticComponent<
  PropsWithoutRef<EngineProps> & RefAttributes<EngineRef>
>;

export type Engine = {
  component: EngineComponent;
  featureComponent: FeatureComponentType;
  clusterComponent: ClusterComponentType;
  sketchComponent: SketchComponentType;
  delegatedDataTypes?: DataType[];
};

export type RequestingRenderMode = -1 | 0 | 1; // -1: force render on every postUpdate, 0: no request to render, 1: request one frame

export type SketchRef = {
  getType: () => SketchType | undefined;
  setType: (type: SketchType | undefined, from?: "editor" | "plugin") => void;
  getOptions: () => SketchOptions;
  overrideOptions: (options: SketchOptions) => void;
  editFeature: (feature: SketchEditingFeature | undefined) => void;
  cancelEdit: (ignoreAutoReSelect?: boolean) => void;
  applyEdit: () => void;
  deleteFeature: (layerId: string, featureId: string) => void;
  onEditFeatureChange: (cb: SketchEditFeatureChangeCb) => void;
};

export type Credit = {
  html?: string;
};
