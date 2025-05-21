import {
  ForwardedRef,
  RefObject,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { v4 as uuid } from "uuid";

import { useWindowEvent } from "../../utils/use-window-event";
import { InteractionModeType } from "../../Visualizer";
import { GeoidRef } from "../Geoid/types";
import { EngineRef, MouseEventProps } from "../types";

import { SPATIALID_DEFAULT_OPTIONS, SPATIALID_LATITUDE_RANGE } from "./constants";
import {
  SpatialIdRef,
  SpatialIdSpacePickingState,
  SpatialIdSpaceType,
  SpatialIdSpaceData,
  SpatialIdPickSpaceOptions,
  VerticalSpaceIndicatorType,
  CoordinateSelectorType,
} from "./types";
import { createSpatialIdSpace, getSpaceData, getVerticalLimits } from "./utils";

type Props = {
  ref: ForwardedRef<SpatialIdRef>;
  engineRef: RefObject<EngineRef>;
  geoidRef: RefObject<GeoidRef>;
  terrainEnabled?: boolean;
  interactionMode?: InteractionModeType;
  overrideInteractionMode?: (mode: InteractionModeType) => void;
  onMount?: () => void;
};

export default ({
  ref,
  engineRef,
  geoidRef,
  terrainEnabled,
  interactionMode,
  overrideInteractionMode,
  onMount,
}: Props) => {
  const [state, setState] = useState<SpatialIdSpacePickingState>("idle");

  const [spatialIdSpaces, setSpatialIdSpaces] = useState<SpatialIdSpaceType[] | null>(null);
  const [verticalSpaceIndicator, setVerticalSpaceIndicator] =
    useState<VerticalSpaceIndicatorType | null>(null);
  const [coordinateSelector, setCoordinateSelector] = useState<CoordinateSelectorType | null>(null);
  const lastCoordinateSelector = useRef<CoordinateSelectorType | null>(null);
  const [spaceSelector, setSpaceSelector] = useState<SpatialIdSpaceType | null>(null);
  const centerGeoidHeightRef = useRef<number | null>(null);

  const [basePosition, setBasePosition] = useState<[number, number, number] | null>(null);
  const [baseCoordinateGeoid, setBaseCoordinateGeoid] = useState<[number, number, number] | null>(
    null,
  );

  const [pickOptions, setPickOptions] =
    useState<Required<SpatialIdPickSpaceOptions>>(SPATIALID_DEFAULT_OPTIONS);

  const verticalLimits = useMemo(
    () => getVerticalLimits(pickOptions.maxHeight, pickOptions.minHeight, pickOptions.zoom),
    [pickOptions.maxHeight, pickOptions.minHeight, pickOptions.zoom],
  );

  const groundIndicators = useMemo(() => {
    const allSpaces = [...(spatialIdSpaces ?? []), ...(spaceSelector ? [spaceSelector] : [])];

    if (!allSpaces) return null;

    const uniqueSpaces = allSpaces.reduce((acc, space) => {
      if (
        !acc.find(
          s =>
            s.space.zfxy.z === space.space.zfxy.z &&
            s.space.zfxy.x === space.space.zfxy.x &&
            s.space.zfxy.y === space.space.zfxy.y,
        )
      ) {
        acc.push(space);
      }
      return acc;
    }, [] as SpatialIdSpaceType[]);

    if (uniqueSpaces.length === 0) return null;

    return uniqueSpaces.map(space => {
      const { wsen } = space;
      return {
        id: uuid(),
        spaceId: space.space.id,
        wsen,
        color: pickOptions.groundIndicatorColor,
      };
    });
  }, [spatialIdSpaces, spaceSelector, pickOptions.groundIndicatorColor]);

  const pickSpace = useCallback(
    (options?: SpatialIdPickSpaceOptions) => {
      setState("coordinate");
      setPickOptions(prev => ({ ...prev, ...options }));
      overrideInteractionMode?.("spatialId");
      setTimeout(() => {
        engineRef.current?.setCursor("crosshair");
      }, 100);
    },
    [engineRef, overrideInteractionMode],
  );

  const interactionModeRef = useRef(interactionMode);
  interactionModeRef.current = interactionMode;

  const finishPicking = useCallback(() => {
    setState("idle");
    setBasePosition(null);
    setBaseCoordinateGeoid(null);
    setSpaceSelector(null);
    setCoordinateSelector(null);
    lastCoordinateSelector.current = null;
    centerGeoidHeightRef.current = null;
    setVerticalSpaceIndicator(null);
    overrideInteractionMode?.(
      interactionModeRef.current === "spatialId"
        ? "default"
        : interactionModeRef.current ?? "default",
    );
    engineRef.current?.setCursor("default");
    engineRef.current?.requestRender();
  }, [overrideInteractionMode, engineRef]);

  const handleMouseUp = useCallback(
    async (props: MouseEventProps) => {
      if (state === "idle") return;
      if (tempSwitchToMoveMode.current) return;

      // handle coordinate picking
      if (state === "coordinate") {
        if (
          !coordinateSelector ||
          props.lat === undefined ||
          props.lng === undefined ||
          props.lat > SPATIALID_LATITUDE_RANGE.max ||
          props.lat < SPATIALID_LATITUDE_RANGE.min
        )
          return;

        setState("fetchingGeoid");

        const { id, wsen, space } = createSpatialIdSpace(
          props.lng,
          props.lat,
          0,
          pickOptions.zoom,
          0,
        );

        requestAnimationFrame(() => {
          engineRef.current?.setCursor("wait");
        });

        const [geoidHeight, centerGeoidHeight] = await Promise.all([
          geoidRef.current?.getGeoidHeight(props.lng, props.lat),
          geoidRef.current?.getGeoidHeight(space.center.lng, space.center.lat),
        ]);

        setTimeout(() => {
          engineRef.current?.setCursor("crosshair");
        }, 100);

        if (geoidHeight === undefined && centerGeoidHeight === undefined) {
          setState("coordinate");
          return;
        }

        // In most case the geoidHeight difference between click point and center is small
        // The API is not that stable, it has NaN for some points for unknown reason
        // Therefore we try use one another if one is NaN
        const appliedGeoidHeight = geoidHeight ?? centerGeoidHeight ?? 0;
        const appliedCenterGeoidHeight = centerGeoidHeight ?? geoidHeight ?? 0;
        centerGeoidHeightRef.current = appliedCenterGeoidHeight;

        setVerticalSpaceIndicator({
          id,
          wsen,
          height: verticalLimits.top + appliedCenterGeoidHeight,
          extrudedHeight: verticalLimits.bottom + appliedCenterGeoidHeight,
          color: pickOptions.verticalSpaceIndicatorColor,
          outlineColor: pickOptions.verticalSpaceIndicatorOutlineColor,
        });

        setBaseCoordinateGeoid([
          props.lng,
          props.lat,
          (terrainEnabled ? props.height ?? 0 : 0) - appliedGeoidHeight,
        ]);

        setBasePosition(engineRef.current?.toXYZ(props.lng, props.lat, props.height ?? 0) ?? null);

        const initialSpaceSelectorSpace = createSpatialIdSpace(
          props.lng,
          props.lat,
          (terrainEnabled ? props.height ?? 0 : 0) - appliedGeoidHeight,
          pickOptions.zoom,
          appliedCenterGeoidHeight,
        );
        setSpaceSelector({
          ...initialSpaceSelectorSpace,
          color: pickOptions.color,
          outlineColor: pickOptions.outlineColor,
        });

        lastCoordinateSelector.current = coordinateSelector;
        setCoordinateSelector(null);

        engineRef.current?.requestRender();
        setState("floor");
      } else if (state === "floor") {
        if (!spaceSelector) return;

        const confirmedSpace: SpatialIdSpaceType = {
          ...spaceSelector,
          id: uuid(),
          color: pickOptions.color,
          outlineColor: pickOptions.outlineColor,
        };

        if (!pickOptions.dataOnly) {
          setSpatialIdSpaces(prev => (prev ? [...prev, confirmedSpace] : [confirmedSpace]));
        }

        finishPicking();

        const spaceData = getSpaceData(confirmedSpace.space);
        onSpacePickEvents.current.forEach(cb => cb(spaceData));
      }
    },
    [
      state,
      terrainEnabled,
      engineRef,
      geoidRef,
      spaceSelector,
      pickOptions,
      verticalLimits,
      coordinateSelector,
      finishPicking,
    ],
  );

  const handleMouseMove = useCallback(
    (props: MouseEventProps) => {
      if (state === "idle") return;
      if (tempSwitchToMoveMode.current) return;

      if (state === "coordinate") {
        if (
          props.lat === undefined ||
          props.lng === undefined ||
          props.lat > SPATIALID_LATITUDE_RANGE.max ||
          props.lat < SPATIALID_LATITUDE_RANGE.min
        )
          return;

        // Coordinate Selector is clamp to ground, we can ignore height
        // The space id is used to identify the selector only
        const newSpace = createSpatialIdSpace(props.lng, props.lat, 0, pickOptions.zoom, 0);

        if (newSpace.space.id === coordinateSelector?.uid) return;
        setCoordinateSelector({
          id: uuid(),
          uid: newSpace.space.id,
          wsen: newSpace.wsen,
          color: pickOptions.selectorColor,
        });
      } else if (state === "floor") {
        if (
          props.x === undefined ||
          props.y === undefined ||
          basePosition === null ||
          baseCoordinateGeoid === null
        )
          return;

        const offset =
          engineRef.current?.getExtrudedHeight(basePosition, [props.x, props.y], true) ?? 0;

        if (
          baseCoordinateGeoid[2] + offset > verticalLimits.top ||
          baseCoordinateGeoid[2] + offset < verticalLimits.bottom
        )
          return;

        const newSpace = createSpatialIdSpace(
          baseCoordinateGeoid[0],
          baseCoordinateGeoid[1],
          baseCoordinateGeoid[2] + offset,
          pickOptions.zoom,
          centerGeoidHeightRef.current ?? 0,
        );

        if (newSpace.space.id === spaceSelector?.space.id) return;

        setSpaceSelector({
          ...newSpace,
          color: pickOptions.selectorColor,
          outlineColor: pickOptions.selectorOutlineColor,
        });
      }
    },
    [
      state,
      spaceSelector,
      basePosition,
      baseCoordinateGeoid,
      engineRef,
      pickOptions,
      verticalLimits,
      coordinateSelector?.uid,
    ],
  );

  const cancel = useCallback(() => {
    if (state === "idle") return;
    if (state === "coordinate" && pickOptions.rightClickToExit) {
      finishPicking();
    } else if (state === "floor") {
      setSpaceSelector(null);
      setBasePosition(null);
      setBaseCoordinateGeoid(null);
      setVerticalSpaceIndicator(null);
      setCoordinateSelector(lastCoordinateSelector.current);
      setState("coordinate");
      centerGeoidHeightRef.current = null;
    }
    engineRef.current?.requestRender();
  }, [state, pickOptions, engineRef, finishPicking]);

  const handleMouseRightClick = useCallback(() => {
    cancel();
  }, [cancel]);

  useWindowEvent("keydown", event => {
    if (event.code === "Escape") {
      cancel();
    }
  });

  // bind mouse events
  const eventsBinded = useRef(false);

  const handleMouseUpRef = useRef(handleMouseUp);
  handleMouseUpRef.current = handleMouseUp;
  const handleMouseUpForRef = useCallback((props: MouseEventProps) => {
    handleMouseUpRef.current(props);
  }, []);

  const handleMouseMoveRef = useRef(handleMouseMove);
  handleMouseMoveRef.current = handleMouseMove;
  const handleMouseMoveForRef = useCallback((props: MouseEventProps) => {
    handleMouseMoveRef.current(props);
  }, []);

  const handleMouseRightClickRef = useRef(handleMouseRightClick);
  handleMouseRightClickRef.current = handleMouseRightClick;
  const handleMouseRightClickForRef = useCallback(() => {
    handleMouseRightClickRef.current();
  }, []);

  useEffect(() => {
    if (eventsBinded.current || !engineRef.current) return;
    eventsBinded.current = true;
    engineRef.current.onMouseUp(handleMouseUpForRef);
    engineRef.current.onMouseMove(handleMouseMoveForRef);
    engineRef.current.onRightClick(handleMouseRightClickForRef);
  }, [engineRef, handleMouseUpForRef, handleMouseMoveForRef, handleMouseRightClickForRef]);

  // cancel picking when interaction mode changes
  const stateRef = useRef(state);
  stateRef.current = state;
  const finishPickingRef = useRef(finishPicking);
  finishPickingRef.current = finishPicking;
  useEffect(() => {
    if (tempSwitchToMoveMode.current) return;
    if (interactionMode !== "spatialId" && stateRef.current !== "idle") {
      finishPickingRef.current();
    }
  }, [interactionMode]);

  // events
  const onSpacePickEvents = useRef<((space: SpatialIdSpaceData) => void)[]>([]);

  const bindEventOnSpacePick = useCallback((cb: (space: SpatialIdSpaceData) => void) => {
    onSpacePickEvents.current.push(cb);
  }, []);

  // ref
  useImperativeHandle(
    ref,
    () => ({
      pickSpace,
      onSpacePick: bindEventOnSpacePick,
      exitPickSpace: finishPicking,
    }),
    [pickSpace, bindEventOnSpacePick, finishPicking],
  );

  // press space to move
  const tempSwitchToMoveMode = useRef(false);
  useEffect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if (e.code === "Space" && stateRef.current !== "idle") {
        tempSwitchToMoveMode.current = true;
        overrideInteractionMode?.("move");
      }
    };
    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  }, [overrideInteractionMode]);

  useEffect(() => {
    return window.addEventListener("keyup", e => {
      if (e.code === "Space" && tempSwitchToMoveMode.current) {
        tempSwitchToMoveMode.current = false;
        overrideInteractionMode?.("spatialId");
      }
    });
  }, [overrideInteractionMode]);

  useEffect(() => {
    onMount?.();
  }, [onMount]);

  return {
    spatialIdSpaces,
    verticalSpaceIndicator,
    coordinateSelector,
    spaceSelector,
    groundIndicators,
  };
};
