import { Atom as Atom_2 } from 'jotai';
import { Cartesian3 } from 'cesium';
import { ComponentType } from 'react';
import { Context } from 'react';
import { CSSProperties } from 'react';
import { Feature as Feature_2 } from 'geojson';
import { ForwardRefExoticComponent } from 'react';
import { LineString } from 'geojson';
import { MemoExoticComponent } from 'react';
import { MultiLineString } from 'geojson';
import { MultiPoint } from 'geojson';
import { MultiPolygon } from 'geojson';
import { MutableRefObject } from 'react';
import { Point } from 'geojson';
import { Polygon as Polygon_2 } from 'geojson';
import { PropsWithoutRef } from 'react';
import { ReactNode } from 'react';
import { RefAttributes } from 'react';
import { RefObject } from 'react';
import { WritableAtom } from 'jotai';

export declare type AmbientOcclusionProperty = {
    enabled?: boolean;
    quality?: "low" | "medium" | "high" | "extreme";
    intensity?: number;
    ambientOcclusionOnly?: boolean;
};

export declare const appearanceKeyObj: {
    [k in keyof AppearanceTypes]: 1;
};

export declare const appearanceKeys: (keyof AppearanceTypes)[];

export declare type AppearanceTypes = {
    marker: MarkerAppearance;
    polyline: PolylineAppearance;
    polygon: PolygonAppearance;
    model: ModelAppearance;
    "3dtiles": Cesium3DTilesAppearance;
    ellipsoid: EllipsoidAppearance;
    ellipse: EllipseAppearance;
    box: BoxAppearance;
    photooverlay: LegacyPhotooverlayAppearance;
    resource: ResourceAppearance;
    raster: RasterAppearance;
    heatMap: HeatMapAppearance;
    frustum: FrustumAppearance;
    transition: TransitionAppearance;
};

declare type Array_2 = any[];
export { Array_2 as Array }

export declare type AssetsCesiumProperty = {
    terrain?: {
        ionAccessToken?: string;
        ionAsset?: string;
        ionUrl?: string;
    };
};

export declare type AssetsProperty = {
    cesium?: AssetsCesiumProperty;
};

export declare type Atom = ReturnType<typeof computeAtom>;

export declare type Bound = {
    east: number;
    north: number;
    south: number;
    west: number;
};

export declare type BoxAppearance = {
    show?: boolean;
    height?: number;
    width?: number;
    length?: number;
    heading?: number;
    pitch?: number;
    roll?: number;
    fillColor?: string;
    outlineColor?: string;
    activeOutlineColor?: string;
    outlineWidth?: number;
    draggableOutlineColor?: string;
    activeDraggableOutlineColor?: string;
    draggableOutlineWidth?: number;
    scalePoint?: boolean;
    axisLine?: boolean;
    pointFillColor?: string;
    pointOutlineColor?: string;
    activePointOutlineColor?: string;
    pointOutlineWidth?: number;
    axisLineColor?: string;
    axisLineWidth?: number;
    allowEnterGround?: boolean;
    cursor?: string;
    activeBox?: boolean;
    activeScalePointIndex?: number;
    activeEdgeIndex?: number;
    near?: number;
    far?: number;
    hideIndicator?: boolean;
    disabledSelection?: boolean;
};

export declare type Camera = {
    lat: number;
    lng: number;
    height: number;
    heading: number;
    pitch: number;
    roll: number;
    fov: number;
    aspectRatio?: number;
};

export declare type CameraLimiterProperty = {
    enabled?: boolean;
    targetArea?: Camera;
    targetWidth?: number;
    targetLength?: number;
    showHelper?: boolean;
};

export declare type CameraOptions = {
    /** Seconds */
    duration?: number;
    easing?: (time: number) => number;
    withoutAnimation?: boolean;
};

/** Represents the camera position and state */
export declare type CameraPosition = {
    /** degrees */
    lat?: number;
    /** degrees */
    lng?: number;
    /** meters */
    height?: number;
    /** radians */
    heading?: number;
    /** radians */
    pitch?: number;
    /** radians */
    roll?: number;
    /** Field of view expressed in radians */
    fov?: number;
    /** Aspect ratio of frustum */
    aspectRatio?: number;
};

export declare type CameraProperty = {
    allowEnterGround?: boolean;
    limiter?: CameraLimiterProperty;
};

export declare type Cesium3DTilesAppearance = {
    show?: boolean;
    color?: string;
    styleUrl?: string;
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    colorBlendMode?: "highlight" | "replace" | "mix";
    edgeWidth?: number;
    edgeColor?: string;
    selectedFeatureColor?: string;
    disableIndexingFeature?: boolean;
    tileset?: string;
    experimental_clipping?: EXPERIMENTAL_clipping;
    pointSize?: number;
    meta?: unknown;
    pbr?: boolean | "withTexture";
    specularEnvironmentMaps?: string;
    sphericalHarmonicCoefficients?: [x: number, y: number, z: number][];
    imageBasedLightIntensity?: number;
    showWireframe?: boolean;
    showBoundingVolume?: boolean;
    cacheBytes?: number;
};

export declare type ClassificationType = "both" | "terrain" | "3dtiles";

export declare function clearAllExpressionCaches(layer: LayerSimple | undefined, feature: Feature | undefined): void;

export declare type Clock = {
    current?: Date;
    start?: Date;
    stop?: Date;
    speed?: number;
    playing?: boolean;
};

export declare type Cluster = {
    id: string;
    property?: ClusterProperty;
    layers?: string[];
};

export declare type ClusterComponentProps = {
    cluster: Cluster;
    property?: ClusterProperty;
    children?: ReactNode;
};

export declare type ClusterComponentType = ComponentType<ClusterComponentProps>;

export declare type ClusterProperty = {
    default?: {
        clusterPixelRange: number;
        clusterMinSize: number;
        clusterLabelTypography?: Typography;
        clusterImage?: string;
        clusterImageHeight?: number;
        clusterImageWidth?: number;
    };
    layers?: {
        layer?: string;
    }[];
};

export declare type ColorTuple = [number, number, number];

declare type Command = {
    type: "setLayer";
    layer?: Layer;
} | {
    type: "writeLayer";
    value: Partial<Pick<LayerSimple, "properties">>;
} | {
    type: "requestFetch";
    range: DataRange;
} | {
    type: "writeFeatures";
    features: Feature[];
} | {
    type: "writeComputedFeatures";
    value: {
        feature: Feature[];
        computed: ComputedFeature[];
        needComputingLayer?: boolean;
    };
} | {
    type: "deleteFeatures";
    features: string[];
} | {
    type: "deleteComputedFeatures";
    features: string[];
} | {
    type: "override";
    overrides?: Record<string, any>;
} | {
    type: "updateDelegatedDataTypes";
    delegatedDataTypes: DataType[];
} | {
    type: "forceUpdateFeatures";
};

export declare type CommonFeature<T extends "feature" | "computedFeature"> = {
    type: T;
    id: string;
    geometry?: Geometry;
    interval?: TimeInterval;
    properties?: any;
    metaData?: {
        description?: string;
    };
    range?: DataRange;
};

declare type CommonProps = {
    isBuilt?: boolean;
    isEditable?: boolean;
    isHidden?: boolean;
    isSelected?: boolean;
    meta?: Record<string, unknown>;
    sketchEditingFeature?: SketchEditingFeature;
};

export declare function computeAtom(cache?: typeof globalDataFeaturesCache): WritableAtom<ComputedLayer | undefined, Command, void>;

export declare type ComputedFeature = CommonFeature<"computedFeature"> & Partial<AppearanceTypes>;

export declare type ComputedLayer = {
    id: string;
    status: ComputedLayerStatus;
    layer: Layer;
    originalFeatures: Feature[];
    features: ComputedFeature[];
    properties?: any;
} & Partial<AppearanceTypes>;

export declare type ComputedLayerStatus = "fetching" | "ready";

export declare type ConditionsExpression = {
    conditions: [string, string][];
};

declare type ControlPointMouseEventHandler = (index: number, isExtrudedPoint: boolean, type: "mousedown" | "click") => void;

export declare function convertLayer(l: Layer): LegacyLayer | undefined;

export declare function convertLegacyCluster(clusters: LegacyCluster[]): Cluster[];

export declare function convertLegacyLayer(l: LegacyLayer | undefined): Layer | undefined;

export declare type Coordinates = LatLngHeight[];

declare type CoreContext = {
    interactionMode?: InteractionModeType;
    selectedLayer?: {
        layerId?: string | undefined;
        featureId?: string | undefined;
        layer?: ComputedLayer | undefined;
        reason?: LayerSelectionReason | undefined;
    };
    selectedComputedFeature?: ComputedFeature | undefined;
    viewport?: Viewport;
    handleCameraForceHorizontalRollChange?: (enable?: boolean) => void;
    handleInteractionModeChange?: (mode?: InteractionModeType | undefined) => void;
    onSketchPluginFeatureCreate?: (cb: SketchEventCallback) => void;
    onSketchPluginFeatureUpdate?: (cb: SketchEventCallback) => void;
    onSketchPluginFeatureDelete?: (cb: SketchEventCallback) => void;
    onSketchTypeChange?: (cb: (type: SketchType | undefined) => void) => void;
    onLayerVisibility?: (cb: (e: LayerVisibilityEvent) => void) => void;
    onLayerLoad?: (cb: (e: LayerLoadEvent) => void) => void;
    onLayerEdit?: (cb: (e: LayerEditEvent) => void) => void;
    onLayerSelectWithRectStart?: (cb: (e: LayerSelectWithRectStart) => void) => void;
    onLayerSelectWithRectMove?: (cb: (e: LayerSelectWithRectMove) => void) => void;
    onLayerSelectWithRectEnd?: (cb: (e: LayerSelectWithRectEnd) => void) => void;
};

export declare const coreContext: Context<CoreContext>;

export declare const CoreVisualizer: MemoExoticComponent<ForwardRefExoticComponent<CoreVisualizerProps & {
children?: ReactNode;
} & RefAttributes<MapRef>>>;

export declare type CoreVisualizerProps = {
    engine?: EngineType;
    isBuilt?: boolean;
    isEditable?: boolean;
    viewerProperty?: ViewerProperty;
    layers?: Layer[];
    clusters?: Cluster[];
    time?: string | Date;
    camera?: Camera;
    interactionMode?: InteractionModeType;
    shouldRender?: boolean;
    meta?: Record<string, unknown>;
    style?: CSSProperties;
    small?: boolean;
    ready?: boolean;
    hiddenLayers?: string[];
    zoomedLayerId?: string;
    displayCredits?: boolean;
    onCameraChange?: (camera: Camera) => void;
    onLayerDrop?: (layerId: string, propertyKey: string, position: LatLng | undefined) => void;
    onLayerSelect?: (layerId: string | undefined, layer: (() => Promise<ComputedLayer | undefined>) | undefined, feature: ComputedFeature | undefined, reason: LayerSelectionReason | undefined) => void;
    onZoomToLayer?: (layerId: string | undefined) => void;
    onMount?: () => void;
    onSketchTypeChangeProp?: (type: SketchType | undefined) => void;
    onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
    onSketchFeatureUpdate?: (feature: SketchFeature | null) => void;
    onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
    onInteractionModeChange?: (mode: InteractionModeType) => void;
    onAPIReady?: () => void;
    onCreditsUpdate?: (credits: Credit[]) => void;
};

export declare type Credit = {
    html?: string;
};

declare type CursorType = "default" | "auto" | "help" | "pointer" | "grab" | "crosshair" | "wait";

export declare type Data = {
    type: DataType;
    url?: string;
    value?: any;
    layers?: string | string[];
    jsonProperties?: string[];
    isSketchLayer?: boolean;
    updateInterval?: number;
    parameters?: Record<string, any>;
    idProperty?: string;
    serviceTokens?: {
        googleMapApiKey?: string;
    };
    time?: {
        property?: string;
        interval?: number;
        updateClockOnLoad?: boolean;
    };
    csv?: {
        idColumn?: string | number;
        latColumn?: string | number;
        lngColumn?: string | number;
        heightColumn?: string | number;
        noHeader?: boolean;
        disableTypeConversion?: boolean;
    };
    geojson?: {
        useAsResource?: boolean;
    };
};

export declare type DataRange = {
    x: number;
    y: number;
    z: number;
};

export declare type DataType = "geojson" | "3dtiles" | "osm-buildings" | "google-photorealistic" | "czml" | "csv" | "wms" | "mvt" | "kml" | "gpx" | "shapefile" | "gtfs" | "gml" | "georss" | "gltf" | "tiles" | "tms" | "heatMap";

export declare type DebugProperty = {
    showGlobeWireframe?: boolean;
    showFramesPerSecond?: boolean;
};

export declare type DefaultInfobox = {
    title?: string;
    content: {
        type: "table";
        value: {
            key: string;
            value: string;
        }[];
    } | {
        type: "html";
        value: string;
    };
};

export declare type ElevationHeatMapProperty = {
    type?: "custom";
    colorLUT?: LUT;
    minHeight?: number;
    maxHeight?: number;
    logarithmic?: boolean;
};

export declare type EllipseAppearance = {
    show?: boolean;
    heightReference?: "none" | "clamp" | "relative";
    classificationType?: ClassificationType;
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    radius?: number;
    fill?: boolean;
    fillColor?: string;
    near?: number;
    far?: number;
};

export declare type EllipsoidAppearance = {
    show?: boolean;
    heightReference?: "none" | "clamp" | "relative";
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    radius?: number;
    fillColor?: string;
    near?: number;
    far?: number;
};

export declare type Engine = {
    component: EngineComponent;
    featureComponent: FeatureComponentType;
    clusterComponent: ClusterComponentType;
    sketchComponent: SketchComponentType;
    delegatedDataTypes?: DataType[];
};

declare type EngineClock = {
    current: Date | undefined;
    start: Date | undefined;
    stop: Date | undefined;
};

export declare type EngineComponent = ForwardRefExoticComponent<PropsWithoutRef<EngineProps> & RefAttributes<EngineRef>>;

export declare type EngineProps = {
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
    onLayerSelect?: (layerId: string | undefined, featureId?: string, options?: LayerSelectionReason, info?: SelectedFeatureInfo) => void;
    onCameraChange?: (camera: Camera) => void;
    onLayerDrag?: (layerId: string, featureId: string | undefined, position: LatLng) => void;
    onLayerDrop?: (layerId: string, featureId: string | undefined, position: LatLng | undefined) => void;
    onLayerEdit?: (e: LayerEditEvent) => void;
    onMount?: () => void;
    onLayerVisibility?: (e: LayerVisibilityEvent) => void;
    onLayerLoad?: (e: LayerLoadEvent) => void;
    onLayerSelectWithRectStart?: (e: LayerSelectWithRectStart) => void;
    onLayerSelectWithRectMove?: (e: LayerSelectWithRectMove) => void;
    onLayerSelectWithRectEnd?: (e: LayerSelectWithRectEnd) => void;
    onCreditsUpdate?: (credits: Credit[]) => void;
};

export declare type EngineRef = {
    name: string;
    requestRender: () => void;
    getViewport: () => Rect | undefined;
    getCamera: () => Camera | undefined;
    getCameraFovInfo: (options: {
        withTerrain?: boolean;
        calcViewSize?: boolean;
    }) => {
        center?: LatLngHeight;
        viewSize?: number;
    } | undefined;
    getLocationFromScreen: (x: number, y: number, withTerrain?: boolean) => LatLngHeight | undefined;
    sampleTerrainHeight: (lng: number, lat: number) => Promise<number | undefined>;
    computeGlobeHeight: (lng: number, lat: number, height?: number) => number | undefined;
    getGlobeHeight: () => number | undefined;
    toXYZ: (lng: number, lat: number, height: number, options?: {
        useGlobeEllipsoid?: boolean;
    }) => [x: number, y: number, z: number] | undefined;
    toLngLatHeight: (x: number, y: number, z: number, options?: {
        useGlobeEllipsoid?: boolean;
    }) => [lng: number, lat: number, height: number] | undefined;
    convertScreenToPositionOffset: (rawPosition: [x: number, y: number, z: number], screenOffset: [x: number, y: number]) => [x: number, y: number, z: number] | undefined;
    isPositionVisible: (position: [x: number, y: number, z: number]) => boolean;
    setView: (camera: CameraPosition) => void;
    toWindowPosition: (position: [x: number, y: number, z: number]) => [x: number, y: number] | undefined;
    getExtrudedHeight: (position: [x: number, y: number, z: number], windowPosition: [x: number, y: number], allowNegative?: boolean) => number | undefined;
    getExtrudedPoint: (position: [x: number, y: number, z: number], extrutedHeight: number) => Position3d | undefined;
    getSurfaceDistance: (point1: Cartesian3, point2: Cartesian3) => number | undefined;
    equalsEpsilon2d: (point1: Position2d, point2: Position2d, relativeEpsilon: number | undefined, absoluteEpsilon: number | undefined) => boolean;
    equalsEpsilon3d: (point1: Position3d, point2: Position3d, relativeEpsilon: number | undefined, absoluteEpsilon: number | undefined) => boolean;
    createGeometry: ({ type, controlPoints, }: {
        type: SketchType;
        controlPoints: Position3d[];
    }) => LineString | Polygon_2 | MultiPolygon | Point | undefined;
    setCursor: (cursor: CursorType) => void;
    flyTo: FlyTo;
    flyToBBox: (bbox: [number, number, number, number], options?: CameraOptions & {
        heading?: number;
        pitch?: number;
        range?: number;
    }) => void;
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
    findComputedFeaturesByIds: (layerId: string, featureId: string[]) => ComputedFeature[] | undefined;
    selectFeatures: (layerId: string, featureId: string[]) => void;
    unselectFeatures: (layerId: string, featureId: string[]) => void;
    pickManyFromViewport: (windowPosition: [x: number, y: number], windowWidth: number, windowHeight: number, condition?: (f: PickedFeature) => boolean) => PickedFeature[] | undefined;
    calcRectangleControlPoint: (p1: Position3d, p2: Position3d, p3: Position3d) => [p1: Position3d, p2: Position3d, p3: Position3d];
    getCredits: () => Credit[] | undefined;
} & MouseEventHandles;

export declare const engines: {
    cesium: Engine;
};

export declare type EngineType = keyof typeof engines;

export declare function evalExpression(expressionContainer: any, layer?: LayerSimple, feature?: Feature): unknown | undefined;

export declare type EvalFeature = (layer: Layer, feature: Feature) => ComputedFeature | undefined;

export declare function evalFeature(layer: Layer, feature: Feature): ComputedFeature | undefined;

export declare type EventCallback<T extends any[] = any[]> = (...args: T) => void;

export declare type EventEmitter<E extends {
    [P in string]: any[];
} = {
    [P in string]: any[];
}> = <T extends keyof E>(type: T, ...args: E[T]) => void;

export declare type Events<E extends {
    [P in string]: any[];
} = {
    [P in string]: any[];
}> = {
    readonly on: <T extends keyof E>(type: T, callback: EventCallback<E[T]>) => void;
    readonly off: <T extends keyof E>(type: T, callback: EventCallback<E[T]>) => void;
    readonly once: <T extends keyof E>(type: T, callback: EventCallback<E[T]>) => void;
};

export declare function events<E extends {
    [P in string]: any[];
} = {
    [P in string]: any[];
}>(): [
Events<E>,
EventEmitter<E>
];

declare type Events_2 = {
    select?: SelectEvent;
};

export declare type EXPERIMENTAL_clipping = {
    useBuiltinBox?: boolean;
    allowEnterGround?: boolean;
    planes?: {
        normal: {
            x: number;
            y: number;
            z: number;
        };
        distance: number;
    }[];
    visible?: boolean;
    location?: LatLngHeight;
    coordinates?: number[];
    /**
     * x-axis
     */
    width?: number;
    /**
     * y-axis
     */
    length?: number;
    /**
     * z-axis
     */
    height?: number;
    heading?: number;
    pitch?: number;
    roll?: number;
    direction?: "inside" | "outside";
    disabledSelection?: boolean;
    draw?: {
        enabled?: boolean;
        surfacePoints?: LatLng[];
        top?: number;
        bottom?: number;
        visible?: boolean;
        direction?: "inside" | "outside";
        style?: {
            fill?: boolean;
            fillColor?: string;
            stroke?: boolean;
            strokeColor?: string;
            strokeWidth?: number;
        };
    };
};

export declare type ExpressionContainer = {
    expression: StyleExpression;
};

export declare type Feature = CommonFeature<"feature">;

export declare const FEATURE_FLAGS: {
    CAMERA_MOVE: number;
    CAMERA_ZOOM: number;
    CAMERA_TILT: number;
    CAMERA_LOOK: number;
    SINGLE_SELECTION: number;
    MULTIPLE_SELECTION: number;
    SKETCH: number;
};

export declare type FeatureComponentProps = {
    layer: ComputedLayer;
    viewerProperty?: ViewerProperty;
    onFeatureRequest?: (range: DataRange) => void;
    onLayerFetch?: (value: Partial<Pick<LayerSimple, "properties">>) => void;
    onFeatureFetch?: (features: Feature[]) => void;
    onComputedFeatureFetch?: (feature: Feature[], computed: ComputedFeature[]) => void;
    onFeatureDelete?: (features: string[]) => void;
    onComputedFeatureDelete?: (features: string[]) => void;
    evalFeature: EvalFeature;
} & CommonProps;

export declare type FeatureComponentType = ComponentType<FeatureComponentProps>;

export declare type FlyTo = (target: string | FlyToDestination, options?: CameraOptions) => void;

export declare type FlyToDestination = {
    /** Degrees */
    lat?: number;
    /** Degrees */
    lng?: number;
    /** Meters */
    height?: number;
    /** Radian */
    heading?: number;
    /** Radian */
    pitch?: number;
    /** Radian */
    roll?: number;
    /** Radian */
    fov?: number;
};

export declare type FogProperty = {
    enabled?: boolean;
    density?: number;
};

export declare type FrustumAppearance = {
    show?: boolean;
    color?: string;
    opacity?: number;
    zoom?: number;
    aspectRatio?: number;
    length?: number;
};

export declare type GeoidProperty = {
    server: {
        url: string;
        geoidProperty: string;
    };
};

declare type GeoidRef = {
    getGeoidHeight: (lat?: number, lng?: number) => Promise<number | undefined>;
};

export declare type Geometry = Point | LineString | Polygon_2 | MultiPoint | MultiLineString | MultiPolygon;

export declare type GeometryOptionsXYZ = {
    type: SketchType;
    controlPoints: Position3d[];
};

export declare function getCompat(l: Layer | undefined): LayerCompat | undefined;

declare const globalDataFeaturesCache: {
    get: Atom_2<(key: string, key2: string) => Feature[] | undefined>;
    set: WritableAtom<null, {
    key: string;
    key2: string;
    value?: Feature[] | undefined;
    }, void> & {
        init: null;
    };
    getAll: Atom_2<(key: string) => Feature[][] | undefined>;
};

export declare type GlobeAtmosphereProperty = {
    enabled?: boolean;
    lightIntensity?: number;
    brightnessShift?: number;
    hueShift?: number;
    saturationShift?: number;
};

export declare type GlobeProperty = {
    baseColor?: string;
    enableLighting?: boolean;
    atmosphere?: GlobeAtmosphereProperty;
    depthTestAgainstTerrain?: boolean;
};

export declare function guessType(url: string | undefined): DataType | undefined;

export declare type HeatMapAppearance = {
    valueMap: string;
    bounds: Bound;
    colorMap?: LUT;
    cropBounds?: Bound;
    width?: number;
    height?: number;
    minValue?: number;
    maxValue?: number;
    opacity?: number;
    contourSpacing?: number;
    contourThickness?: number;
    contourAlpha?: number;
    logarithmic?: boolean;
};

export declare type ImageBasedLighting = {
    enabled?: boolean;
    intensity?: number;
    specularEnvironmentMaps?: string;
    sphericalHarmonicCoefficients?: [number, number, number][];
};

export declare type IndicatorProperty = {
    type?: "default" | "crosshair" | "custom";
    image?: string;
    imageScale?: number;
};

declare type Infobox<BP = any> = {
    featureId?: string;
    property?: InfoboxProperty;
    blocks?: InfoboxBlock<BP>[];
};

declare type InfoboxBlock<P = any> = {
    id: string;
    name?: string;
    pluginId?: string;
    extensionId?: string;
    property?: P;
    propertyId?: string;
};

declare type InfoboxProperty = {
    default?: {
        enabled?: PropertyItem<boolean>;
        position?: PropertyItem<"right" | "left">;
        padding?: PropertyItem<Spacing>;
        gap?: PropertyItem<number>;
    };
    defaultContent?: "description" | "attributes";
};

export declare const INTERACTION_MODES: Record<InteractionModeType, number>;

export declare type InteractionModeType = "default" | "move" | "selection" | "sketch" | "spatialId";

export declare function isSketchType(value: unknown): value is SketchType;

export declare type LatLng = {
    lat: number;
    lng: number;
};

export declare type LatLngHeight = {
    lat: number;
    lng: number;
    height: number;
};

export declare type Layer = LayerSimple | LayerGroup;

export declare type LayerAppearance<T> = {
    [K in keyof T]?: T[K] | LayerAppearance<T[K]> | ExpressionContainer;
};

export declare type LayerAppearanceTypes = {
    [K in keyof AppearanceTypes]: LayerAppearance<AppearanceTypes[K]>;
};

export declare type LayerCommon = {
    id: string;
    title?: string;
    /** default is true */
    visible?: boolean;
    infobox?: Infobox;
    tags?: Tag[];
    creator?: string;
    compat?: LayerCompat;
    _updateStyle?: number;
};

export declare type LayerCompat = {
    extensionId?: string;
    property?: any;
    propertyId?: string;
};

export declare type LayerEditEvent = {
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

export declare type LayerGroup = {
    type: "group";
    children: Layer[];
} & LayerCommon;

export declare type LayerLoadEvent = {
    layerId: string | undefined;
};

export declare type LayerSelectionReason = {
    reason?: string;
    defaultInfobox?: DefaultInfobox;
};

export declare type LayerSelectWithRect = MouseEventProps & {
    pressedKey?: "shift";
};

export declare type LayerSelectWithRectEnd = LayerSelectWithRect & {
    features: PickedFeature[] | undefined;
    isClick: boolean;
};

export declare type LayerSelectWithRectMove = LayerSelectWithRect & {
    startX?: number;
    startY?: number;
    width?: number;
    height?: number;
};

export declare type LayerSelectWithRectStart = LayerSelectWithRect;

export declare type LayerSimple = {
    type: "simple";
    data?: Data;
    properties?: any;
    defines?: Record<string, string>;
    events?: Events_2;
    layerStyleId?: string;
} & Partial<LayerAppearanceTypes> & LayerCommon;

export declare type LayersRef = {
    findById: (id: string) => LazyLayer | undefined;
    findByIds: (...ids: string[]) => (LazyLayer | undefined)[];
    add: (layer: NaiveLayer) => LazyLayer | undefined;
    addAll: (...layers: NaiveLayer[]) => (LazyLayer | undefined)[];
    replace: (...layers: Layer[]) => void;
    override: (id: string, layer?: (Partial<Layer> & {
        property?: any;
    }) | null) => void;
    deleteLayer: (...ids: string[]) => void;
    isLayer: (obj: any) => obj is LazyLayer;
    isComputedLayer: (obj: any) => obj is ComputedLayer;
    isTempLayer: (layerId?: string) => boolean;
    layers: () => LazyLayer[];
    walk: <T>(fn: (layer: LazyLayer, index: number, parents: LazyLayer[]) => T | void) => T | undefined;
    find: (fn: (layer: LazyLayer, index: number, parents: LazyLayer[]) => boolean) => LazyLayer | undefined;
    findAll: (fn: (layer: LazyLayer, index: number, parents: LazyLayer[]) => boolean) => LazyLayer[];
    findByTags: (...tagIds: string[]) => LazyLayer[];
    findByTagLabels: (...tagLabels: string[]) => LazyLayer[];
    hide: (...layers: string[]) => void;
    show: (...layers: string[]) => void;
    select: (layerId: string | undefined, reason?: LayerSelectionReason, info?: SelectedFeatureInfo) => void;
    selectFeature: (layerId: string | undefined, featureId: string | undefined, reason?: LayerSelectionReason, info?: SelectedFeatureInfo) => void;
    selectFeatures: (layers: {
        layerId?: string;
        featureId?: string[];
    }[], reason?: LayerSelectionReason, info?: SelectedFeatureInfo) => void;
    selectedLayer: () => LazyLayer | undefined;
    selectedFeature: () => ComputedFeature | undefined;
    overriddenLayers: () => OverriddenLayer[];
};

export declare type LayerVisibilityEvent = {
    layerId: string | undefined;
};

/**
 * Same as a Layer, but all fields except id is lazily evaluated,
 * in order to reduce unnecessary sending and receiving of data to and from
 * QuickJS (a plugin runtime) and to improve performance.
 */
export declare type LazyLayer = Readonly<Layer> & {
    computed?: Readonly<ComputedLayer>;
    isTempLayer?: boolean;
    pluginId?: string;
    extensionId?: string;
    property?: any;
    propertyId?: string;
    isVisible?: boolean;
};

export declare type LegacyCluster = {
    id: string;
    default?: {
        clusterPixelRange: number;
        clusterMinSize: number;
        clusterLabelTypography?: Typography_2;
        clusterImage?: string;
        clusterImageHeight?: number;
        clusterImageWidth?: number;
    };
    layers?: {
        layer?: string;
    }[];
};

export declare type LegacyLayer<P = any, IBP = any> = {
    id: string;
    type?: string;
    pluginId?: string;
    extensionId?: string;
    title?: string;
    property?: P;
    infobox?: Infobox<IBP>;
    isVisible?: boolean;
    propertyId?: string;
    tags?: Tag[];
    readonly children?: LegacyLayer[];
    creator?: string;
};

export declare type LegacyPhotooverlayAppearance = {
    show?: boolean;
    location?: LatLng;
    height?: number;
    heightReference?: "none" | "clamp" | "relative";
    camera?: Camera;
    image?: string;
    imageSize?: number;
    imageHorizontalOrigin?: "left" | "center" | "right";
    imageVerticalOrigin?: "top" | "center" | "baseline" | "bottom";
    imageCrop?: "none" | "rounded" | "circle";
    imageShadow?: boolean;
    imageShadowColor?: string;
    imageShadowBlur?: number;
    imageShadowPositionX?: number;
    imageShadowPositionY?: number;
    photoOverlayImage?: string;
    photoOverlayDescription?: string;
    near?: number;
    far?: number;
};

export declare type LightProperty = {
    type?: "sunLight" | "directionalLight";
    direction?: [x: number, y: number, z: number];
    color?: string;
    intensity?: number;
};

export declare type LookAtDestination = {
    /** Degrees */
    lat?: number;
    /** Degrees */
    lng?: number;
    /** Meters */
    height?: number;
    /** Radian */
    heading?: number;
    /** Radian */
    pitch?: number;
    /** Radian */
    range?: number;
    /** Radian */
    fov?: number;
    /** Meters */
    radius?: number;
};

export declare type LUT = readonly ColorTuple[];

declare const Map_2: ForwardRefExoticComponent<    {
engines?: Record<string, Engine> | undefined;
engine?: string | undefined;
onAPIReady?: (() => void) | undefined;
} & Omit<Props_2, "Feature" | "viewerProperty" | "selectionReason" | "delegatedDataTypes" | "clusterComponent" | "selectedLayerId"> & Omit<EngineProps, "onLayerSelect" | "selectedLayerId" | "layerSelectionReason"> & Omit<SketchProps, "engineRef" | "SketchComponent" | "layersRef"> & RefAttributes<MapRef>>;
export { Map_2 as Map }

export declare type MapRef = {
    engine: WrappedRef<EngineRef>;
    layers: WrappedRef<LayersRef>;
    sketch: WrappedRef<SketchRef>;
    spatialId?: WrappedRef<SpatialIdRef>;
    geoid: WrappedRef<GeoidRef>;
    timeline?: TimelineManagerRef;
};

export declare type MarkerAppearance = {
    show?: boolean;
    height?: number;
    heightReference?: "none" | "clamp" | "relative";
    style?: "none" | "point" | "image";
    pointSize?: number;
    pointColor?: string;
    pointOutlineColor?: string;
    pointOutlineWidth?: number;
    image?: string;
    imageSize?: number;
    imageSizeInMeters?: boolean;
    imageHorizontalOrigin?: "left" | "center" | "right";
    imageVerticalOrigin?: "top" | "center" | "baseline" | "bottom";
    imageColor?: string;
    imageCrop?: "none" | "rounded" | "circle";
    imageShadow?: boolean;
    imageShadowColor?: string;
    imageShadowBlur?: number;
    imageShadowPositionX?: number;
    imageShadowPositionY?: number;
    label?: boolean;
    labelText?: string;
    labelPosition?: "left" | "right" | "top" | "bottom" | "lefttop" | "leftbottom" | "righttop" | "rightbottom";
    labelTypography?: Typography;
    labelBackground?: boolean;
    labelBackgroundColor?: string;
    labelBackgroundPaddingHorizontal?: number;
    labelBackgroundPaddingVertical?: number;
    extrude?: boolean;
    near?: number;
    far?: number;
    pixelOffset?: [number, number];
    eyeOffset?: [number, number, number];
    hideIndicator?: boolean;
    selectedFeatureColor?: string;
};

export declare function mergeEvents<E extends {
    [x: string]: any[];
} = {
    [x: string]: any[];
}>(source: Events<E>, dest: EventEmitter<E>, types: (keyof E)[]): () => void;

export declare type ModelAppearance = {
    show?: boolean;
    model?: string;
    url?: string;
    heightReference?: "none" | "clamp" | "relative";
    heading?: number;
    pitch?: number;
    roll?: number;
    scale?: number;
    maximumScale?: number;
    minimumPixelSize?: number;
    animation?: boolean;
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    colorBlend?: "none" | "highlight" | "replace" | "mix";
    color?: string;
    colorBlendAmount?: number;
    lightColor?: string;
    silhouette?: boolean;
    silhouetteColor?: string;
    bearing?: number;
    silhouetteSize?: number;
    near?: number;
    far?: number;
    pbr?: boolean | "withTexture";
    specularEnvironmentMaps?: string;
    sphericalHarmonicCoefficients?: [x: number, y: number, z: number][];
    imageBasedLightIntensity?: number;
};

export declare type ModifiedCameraEventType = {
    eventType: OverideCameraEventType;
    modifier: OverideKeyboardEventModifier;
};

export declare type MoonProperty = {
    show?: boolean;
};

export declare type MouseEventCallback = (props: MouseEventProps) => void;

export declare type MouseEventCallbacks = {
    [key in keyof MouseEvents]: MouseEvents[key][];
};

export declare type MouseEventHandles = {
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

export declare type MouseEventProps = {
    x?: number;
    y?: number;
    lat?: number;
    lng?: number;
    height?: number;
    layerId?: string;
    delta?: number;
};

export declare type MouseEvents = {
    [key in MouseEventTypes]: MouseEventCallback;
} & {
    wheel: MouseWheelEventCallback;
};

export declare type MouseEventTypes = "click" | "doubleClick" | "mouseDown" | "mouseUp" | "rightClick" | "rightDown" | "rightUp" | "middleClick" | "middleDown" | "middleUp" | "mouseMove" | "mouseEnter" | "mouseLeave" | "wheel";

export declare type MouseWheelEventCallback = (props: MouseEventProps) => void;

export declare type NaiveBlock<P = any> = Omit<InfoboxBlock<P>, "id">;

export declare type NaiveInfobox = Omit<Infobox, "id" | "blocks"> & {
    blocks?: NaiveBlock[];
};

/** Same as a Layer, but its ID is unknown. */
export declare type NaiveLayer = NaiveLayerSimple | NaiveLayerGroup;

export declare type NaiveLayerGroup = Omit<LayerGroup, "id" | "children" | "infobox"> & {
    infobox?: NaiveInfobox;
    children?: NaiveLayer[];
};

export declare type NaiveLayerSimple = Omit<LayerSimple, "id" | "infobox"> & {
    infobox?: NaiveInfobox;
};

export declare type OnLayerSelectType = (layerId: string | undefined, featureId: string | undefined, layer: (() => Promise<ComputedLayer | undefined>) | undefined, reason: LayerSelectionReason | undefined, info: SelectedFeatureInfo | undefined) => void;

declare type OpenUrlEvent = {
    url?: string;
    urlKey?: string;
};

export declare type OverideCameraEventType = "left_drag" | "right_drag" | "middle_drag" | "wheel" | "pinch";

export declare type OverideKeyboardEventModifier = "ctrl" | "shift" | "alt";

export declare type OverriddenLayer = Omit<Layer, "type" | "children">;

declare type PauseCommand = {
    cmd: "PAUSE";
};

declare type PickedFeature = ComputedFeature & {
    layerId?: string;
};

export declare type Plane = {
    location: LatLngHeight;
    width: number;
    height: number;
    length: number;
    heading: number;
    pitch: number;
};

declare type PlayCommand = {
    cmd: "PLAY";
};

export declare type Polygon = LatLngHeight[][];

export declare type PolygonAppearance = {
    show?: boolean;
    fill?: boolean;
    fillColor?: string;
    stroke?: boolean;
    strokeColor?: string;
    strokeWidth?: number;
    height?: number;
    heightReference?: "none" | "clamp" | "relative";
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    lineJoin?: CanvasLineJoin;
    near?: number;
    far?: number;
    extrudedHeight?: number;
    classificationType?: ClassificationType;
    hideIndicator?: boolean;
    selectedFeatureColor?: string;
};

export declare type PolylineAppearance = {
    show?: boolean;
    clampToGround?: boolean;
    strokeColor?: string;
    strokeWidth?: number;
    shadows?: "disabled" | "enabled" | "cast_only" | "receive_only";
    near?: number;
    far?: number;
    classificationType?: ClassificationType;
    hideIndicator?: boolean;
    selectedFeatureColor?: string;
};

export declare type Position2d = [x: number, y: number];

export declare type Position3d = [x: number, y: number, z: number];

declare type PropertyItem<T> = {
    type?: string;
    ui?: string;
    title?: string;
    description?: string;
    value?: T;
    min?: number;
    max?: number;
    choices?: {
        [key: string]: string;
    }[];
};

export declare type Props = {
    engines?: Record<string, Engine>;
    engine?: string;
    onAPIReady?: () => void;
} & Omit<Props_2, "Feature" | "clusterComponent" | "selectionReason" | "delegatedDataTypes" | "selectedLayerId" | "viewerProperty"> & Omit<EngineProps, "onLayerSelect" | "layerSelectionReason" | "selectedLayerId"> & Omit<SketchProps, "layersRef" | "engineRef" | "SketchComponent">;

declare type Props_2 = Omit<Props_3, "atomMap" | "isHidden" | "selectedLayerId"> & {
    selectedLayer?: {
        layerId?: string;
        featureId?: string;
        reason?: LayerSelectionReason;
    };
    hiddenLayers?: string[];
    viewerProperty?: ViewerProperty;
    requestingRenderMode?: MutableRefObject<RequestingRenderMode>;
    engineRef?: RefObject<EngineRef>;
    onLayerSelect?: (layerId: string | undefined, featureId: string | undefined, layer: (() => Promise<ComputedLayer | undefined>) | undefined, reason: LayerSelectionReason | undefined, info: SelectedFeatureInfo | undefined) => void;
    onMount?: () => void;
};

declare type Props_3 = {
    layers?: Layer[];
    atomMap?: Map<string, Atom>;
    overrides?: Record<string, Record<string, any>>;
    selectedLayer?: {
        layerId?: string;
        featureId?: string;
    };
    isHidden?: (id: string) => boolean;
    clusters?: Cluster[];
    delegatedDataTypes?: DataType[];
    viewerProperty?: ViewerProperty;
    clusterComponent?: ClusterComponentType;
    Feature?: Props_4["Feature"];
} & Omit<CommonProps, "isSelected" | "isHidden" | "selectedFeatureId">;

declare type Props_4 = {
    layer?: Layer;
    atom?: Atom;
    overrides?: Record<string, any>;
    delegatedDataTypes?: DataType[];
    viewerProperty?: ViewerProperty;
    selectedFeatureId?: string;
    /** Feature component should be injected by a map engine. */
    Feature?: ComponentType<FeatureComponentProps>;
} & CommonProps;

export declare type RasterAppearance = {
    show?: boolean;
    minimumLevel?: number;
    maximumLevel?: number;
    credit?: string;
    alpha?: number;
    hideIndicator?: boolean;
    bounds?: string;
};

export declare type Rect = {
    west: number;
    south: number;
    east: number;
    north: number;
};

export declare type RenderPeropty = {
    antialias?: "low" | "medium" | "high" | "extreme";
    ambientOcclusion?: AmbientOcclusionProperty;
};

export declare type RequestingRenderMode = -1 | 0 | 1;

export declare type ResourceAppearance = {
    show?: boolean;
    url?: string;
    type?: "geojson" | "kml" | "czml" | "auto";
    clampToGround?: boolean;
    markerSize?: number;
    markerColor?: string;
    stroke?: string;
    strokeWidth?: number;
    fill?: string;
    hideIndicator?: boolean;
};

export declare type SceneMode = "3d" | "2d" | "columbus";

export declare type SceneProperty = {
    backgroundColor?: string;
    mode?: SceneMode;
    verticalExaggeration?: number;
    verticalExaggerationRelativeHeight?: number;
    vr?: boolean;
    light?: LightProperty;
    shadow?: ShadowProperty;
    imageBasedLighting?: ImageBasedLighting;
};

export declare type ScreenSpaceCameraControllerOptions = {
    zoomEventTypes?: (OverideCameraEventType | ModifiedCameraEventType)[];
    rotateEventTypes?: (OverideCameraEventType | ModifiedCameraEventType)[];
    tiltEventTypes?: (OverideCameraEventType | ModifiedCameraEventType)[];
    lookEventTypes?: (OverideCameraEventType | ModifiedCameraEventType)[];
    translateEventTypes?: (OverideCameraEventType | ModifiedCameraEventType)[];
    minimumZoomDistance?: number;
    maximumZoomDistance?: number;
    enableCollisionDetection?: boolean;
};

export declare type SelectedFeatureInfo = {
    feature?: ComputedFeature;
};

declare type SelectEvent = {
    openUrl?: OpenUrlEvent;
};

declare type SetOptionsCommand = {
    cmd: "SET_OPTIONS";
    payload: Partial<Pick<TimelineOptions, "multiplier" | "rangeType" | "stepType">>;
};

declare type SetTimeCommand = {
    cmd: "SET_TIME";
    payload: {
        current: Date | string;
        start: Date | string;
        stop: Date | string;
    };
};

export declare type ShadowMapProperty = {
    size?: 1024 | 2048 | 4096;
    softShadows?: boolean;
    darkness?: number;
    maximumDistance?: number;
};

export declare type ShadowProperty = {
    enabled?: boolean;
    darkness?: number;
    shadowMap?: ShadowMapProperty;
};

export declare type SketchAppearance = Partial<LayerAppearanceTypes>;

declare type SketchComponentProps = {
    geometryOptions?: {
        type: SketchType;
        controlPoints: readonly Position3d[];
    } | null;
    extrudedHeight?: number;
    extrudedPoint?: Position3d;
    centroidBasePoint?: Position3d;
    centroidExtrudedPoint?: Position3d;
    disableShadow?: boolean;
    color?: string;
    isEditing?: boolean;
    catchedControlPointIndex?: number;
    catchedExtrudedPoint?: boolean;
    selectedControlPointIndex?: number;
    handleControlPointMouseEvent?: ControlPointMouseEventHandler;
    handleAddControlPoint?: (position: Position3d, index: number) => void;
};

export declare type SketchComponentType = ComponentType<SketchComponentProps>;

export declare type SketchEditFeatureChangeCb = (feature: SketchEditingFeature | undefined) => void;

export declare type SketchEditingFeature = {
    layerId: string;
    feature: ComputedFeature;
};

export declare type SketchEventCallback = (event: SketchEventProps) => void;

export declare type SketchEventProps = {
    layerId?: string;
    featureId?: string;
    feature?: SketchFeature;
};

export declare type SketchFeature = Feature_2<Polygon_2 | MultiPolygon | Point | LineString, {
    id: string;
    type: SketchType;
    positions: readonly Position3d[];
    extrudedHeight: number;
}>;

export declare type SketchOptions = {
    color?: string;
    appearance?: SketchAppearance;
    dataOnly?: boolean;
    disableShadow?: boolean;
    rightClickToAbort?: boolean;
    autoResetInteractionMode?: boolean;
    useCentroidExtrudedHeight?: boolean;
};

export declare type SketchProps = {
    layersRef: RefObject<LayersRef>;
    engineRef: RefObject<EngineRef>;
    SketchComponent?: SketchComponentType;
    selectedFeature?: Feature;
    interactionMode?: InteractionModeType;
    overrideInteractionMode?: (mode: InteractionModeType) => void;
    onSketchTypeChange?: (type: SketchType | undefined, from?: "editor" | "plugin") => void;
    onSketchFeatureCreate?: (feature: SketchFeature | null) => void;
    onSketchPluginFeatureCreate?: (props: SketchEventProps) => void;
    onSketchFeatureUpdate?: (feature: SketchFeature | null) => void;
    onSketchPluginFeatureUpdate?: (props: SketchEventProps) => void;
    onSketchFeatureDelete?: (layerId: string, featureId: string) => void;
    onSketchPluginFeatureDelete?: (props: {
        layerId: string;
        featureId: string;
    }) => void;
    onLayerSelect?: OnLayerSelectType;
    sketchEditingFeature?: SketchEditingFeature;
    onSketchEditFeature?: (feature: SketchEditingFeature | undefined) => void;
    onMount?: () => void;
};

export declare type SketchRef = {
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

export declare type SketchType = "marker" | "polyline" | "circle" | "rectangle" | "polygon" | "extrudedCircle" | "extrudedRectangle" | "extrudedPolygon";

export declare type SkyAtmosphereProperty = {
    show?: boolean;
    lightIntensity?: number;
    saturationShift?: number;
    brightnessShift?: number;
};

export declare type SkyBoxProperty = {
    show?: boolean;
};

export declare type SkyProperty = {
    skyBox?: SkyBoxProperty;
    sun?: SunProperty;
    moon?: MoonProperty;
    fog?: FogProperty;
    skyAtmosphere?: SkyAtmosphereProperty;
};

export declare type Spacing = {
    bottom: number;
    left: number;
    right: number;
    top: number;
};

declare type SpatialIdPickSpaceOptions = {
    zoom?: number;
    maxHeight?: number;
    minHeight?: number;
    dataOnly?: boolean;
    rightClickToExit?: boolean;
    color?: string;
    outlineColor?: string;
    groundIndicatorColor?: string;
    selectorColor?: string;
    selectorOutlineColor?: string;
    verticalSpaceIndicatorColor?: string;
    verticalSpaceIndicatorOutlineColor?: string;
};

declare type SpatialIdRef = {
    pickSpace: (options?: SpatialIdPickSpaceOptions) => void;
    exitPickSpace: () => void;
    onSpacePick: (cb: (space: SpatialIdSpaceData) => void) => void;
};

declare type SpatialIdSpaceData = {
    id: string;
    center: {
        lat: number;
        lng: number;
        alt?: number;
    };
    alt: number;
    zoom: number;
    zfxy: {
        z: number;
        f: number;
        x: number;
        y: number;
    };
    zfxyStr: string;
    tilehash: string;
    hilbertTilehash: string;
    hilbertIndex: string;
    vertices: [number, number, number][];
};

export declare type StyleExpression = ConditionsExpression | string;

export declare type SunProperty = {
    show?: boolean;
};

export declare type Tag = {
    id: string;
    label: string;
    tags?: Tag[];
};

export declare type TerrainProperty = {
    enabled?: boolean;
    type?: "cesium" | "arcgis" | "cesiumion";
    url?: string;
    normal?: boolean;
    elevationHeatMap?: ElevationHeatMapProperty;
};

export declare type TickEvent = (cb: TickEventCallback) => void;

declare type TickEvent_2 = (cb: TickEventCallback_2) => void;

export declare type TickEventCallback = (current: Date, clock: {
    start: Date;
    stop: Date;
}) => void;

declare type TickEventCallback_2 = (current: Date, clock: {
    start: Date;
    stop: Date;
}) => void;

export declare type TileLabelProperty = {
    id: string;
    labelType: "japan_gsi_optimal_bvmap";
    style: Record<string, any>;
    near?: number;
    far?: number;
};

export declare type TileProperty = {
    id: string;
    type?: string;
    url?: string;
    opacity?: number;
    zoomLevel?: number[];
    zoomLevelForURL?: number[];
    heatmap?: boolean;
};

export declare type TimeInterval = [start: Date, end?: Date];

export declare type Timeline = {
    startTime?: string;
    endTime?: string;
    currentTime?: string;
};

declare type Timeline_2 = {
    current: Date;
    start: Date;
    stop: Date;
};

declare type TimelineCommit = (PlayCommand | PauseCommand | SetTimeCommand | SetOptionsCommand) & {
    committer: TimelineCommitter;
};

export declare type TimelineCommitter = {
    source: "widgetContext" | "pluginAPI" | "featureResource" | "storyTimelineBlock" | "storyPage" | "initialize";
    id?: string;
};

declare type TimelineManager = {
    readonly timeline: EngineClock;
    readonly options: TimelineOptions;
    readonly computedTimeline: Timeline_2;
    commit: (commit: TimelineCommit) => void;
    onTick: TickEvent_2;
    offTick: TickEvent_2;
    onCommit: (cb: (committer: TimelineCommitter) => void) => void;
    offCommit: (cb: (committer: TimelineCommitter) => void) => void;
    handleTick: (d: Date, clock: {
        start: Date;
        stop: Date;
    }) => void;
    tick: (() => Date | void | undefined) | undefined;
};

export declare type TimelineManagerRef = MutableRefObject<TimelineManager | undefined>;

declare type TimelineOptions = {
    animation: boolean;
    stepType: "rate" | "fixed";
    multiplier: number;
    rangeType?: "unbounded" | "clamped" | "bounced";
};

export declare type TransitionAppearance = {
    useTransition?: boolean;
    translate?: [lng: number, lat: number, height: number];
    rotate?: [heading: number, pitch: number, roll: number];
    scale?: [x: number, y: number, z: number];
};

export declare type Typography = {
    fontFamily?: string;
    fontSize?: number;
    fontWeight?: number;
    color?: string;
    textAlign?: "left" | "center" | "right" | "justify" | "justify_all";
    bold?: boolean;
    italic?: boolean;
    underline?: boolean;
};

declare type Typography_2 = {
    fontFamily?: string;
    fontSize?: number;
    fontWeight?: number;
    color?: string;
    textAlign?: "left" | "center" | "right" | "justify" | "justify_all";
    bold?: boolean;
    italic?: boolean;
    underline?: boolean;
};

export declare type Undefinable<T extends object> = {
    [K in keyof T]: T[K] extends object ? T[K] extends (...args: any[]) => any ? T[K] | undefined : Undefinable<T[K]> | undefined : T[K] | undefined;
};

export declare function useGet<T>(value: T): () => T;

export declare const useVisualizer: () => RefObject<MapRef>;

export declare type ValueType = keyof ValueTypes;

export declare type ValueTypes = {
    string: string;
    number: number;
    bool: boolean;
    latlng: LatLng;
    latlngheight: LatLngHeight;
    url: string;
    camera: Camera;
    typography: Typography;
    coordinates: Coordinates;
    polygon: Polygon;
    rect: Rect;
    ref: string;
    tiletype: string;
    spacing: Spacing;
    array: Array_2;
    timeline: Timeline;
};

export declare type ViewerProperty = {
    globe?: GlobeProperty;
    geoid?: GeoidProperty;
    terrain?: TerrainProperty;
    scene?: SceneProperty;
    tiles?: TileProperty[];
    tileLabels?: TileLabelProperty[];
    sky?: SkyProperty;
    camera?: CameraProperty;
    render?: RenderPeropty;
    assets?: AssetsProperty;
    debug?: DebugProperty;
    indicator?: IndicatorProperty;
};

export declare type Viewport = {
    width: number | undefined;
    height: number | undefined;
    isMobile: boolean | undefined;
    query: Record<string, string>;
};

export declare type VisualizerContext = RefObject<MapRef>;

export declare type WrappedRef<T> = {
    [P in keyof T as T[P] extends (...args: any[]) => any ? P : never]: T[P] extends (...args: infer A) => infer R ? (...args: A) => R | undefined : never;
};

export { }


declare global {
    interface Window {
        REEARTH_E2E_ACCESS_TOKEN?: string;
        REEARTH_E2E_CESIUM_VIEWER?: any;
    }
}


declare module "@cesium/engine" {
    namespace SceneTransforms {
        function transformWindowToDrawingBuffer(scene: Scene, windowPosition: Cartesian2, result?: Cartesian2): Cartesian2;
    }
}


    namespace SceneTransforms {
        function transformWindowToDrawingBuffer(scene: Scene, windowPosition: Cartesian2, result?: Cartesian2): Cartesian2;
    }


declare global {
    namespace Vi {
        interface JestAssertion<T = any> extends jest.Matchers<void, T>, EmotionMatchers {
            toHaveStyleRule: EmotionMatchers["toHaveStyleRule"];
        }
    }
}


    namespace Vi {
        interface JestAssertion<T = any> extends jest.Matchers<void, T>, EmotionMatchers {
            toHaveStyleRule: EmotionMatchers["toHaveStyleRule"];
        }
    }

