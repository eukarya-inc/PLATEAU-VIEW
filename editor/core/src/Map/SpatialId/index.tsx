import { forwardRef, ForwardRefRenderFunction, RefObject } from "react";

import {
  CoordinateIndicator,
  SpatialIdSpace,
  VerticalSpaceIndicator,
} from "../../engines/Cesium/SpatialId";
import { InteractionModeType } from "../../Visualizer";
import { GeoidRef } from "../Geoid/types";
import { EngineRef } from "../types";

import useHooks from "./hooks";
import { SpatialIdRef } from "./types";

type SpatialIdProps = {
  engineRef: RefObject<EngineRef>;
  geoidRef: RefObject<GeoidRef>;
  terrainEnabled?: boolean;
  interactionMode?: InteractionModeType;
  overrideInteractionMode?: (mode: InteractionModeType) => void;
  onMount?: () => void;
};

const SpatialId: ForwardRefRenderFunction<SpatialIdRef, SpatialIdProps> = (
  { engineRef, geoidRef, terrainEnabled, interactionMode, overrideInteractionMode, onMount },
  ref,
) => {
  const {
    spatialIdSpaces,
    verticalSpaceIndicator,
    coordinateSelector,
    spaceSelector,
    groundIndicators,
  } = useHooks({
    ref,
    engineRef,
    geoidRef,
    terrainEnabled,
    interactionMode,
    overrideInteractionMode,
    onMount,
  });

  return (
    <>
      {spatialIdSpaces &&
        spatialIdSpaces.length > 0 &&
        spatialIdSpaces.map(space => <SpatialIdSpace key={space.id} space={space} />)}
      {groundIndicators &&
        groundIndicators.length > 0 &&
        groundIndicators.map(indicator => (
          <CoordinateIndicator key={indicator.id} wsen={indicator.wsen} color={indicator.color} />
        ))}
      {verticalSpaceIndicator && <VerticalSpaceIndicator indicator={verticalSpaceIndicator} />}
      {coordinateSelector && (
        <CoordinateIndicator wsen={coordinateSelector.wsen} color={coordinateSelector.color} />
      )}
      {spaceSelector && <SpatialIdSpace space={spaceSelector} />}
    </>
  );
};

export default forwardRef(SpatialId);
