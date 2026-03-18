import { Viewer } from "cesium";
import { MutableRefObject, RefObject, useCallback, useEffect, useRef } from "react";
import { CesiumComponentRef } from "resium";

import { RequestingRenderMode, ViewerProperty } from "../../../Map";
import { FORCE_REQUEST_RENDER, NO_REQUEST_RENDER, REQUEST_RENDER_ONCE } from "../../../Map/hooks";

export default ({
  cesium,
  requestingRenderMode,
  isLayerDragging,
  shouldRender,
  property,
}: {
  cesium: RefObject<CesiumComponentRef<Viewer>>;
  requestingRenderMode?: MutableRefObject<RequestingRenderMode>;
  isLayerDragging?: boolean;
  shouldRender?: boolean;
  property?: ViewerProperty;
}) => {
  // explicit rendering
  const explicitRender = useCallback(() => {
    const viewer = cesium.current?.cesiumElement;
    if (!requestingRenderMode?.current || !viewer || viewer.isDestroyed()) return;
    viewer.scene.requestRender();
    if (requestingRenderMode.current === REQUEST_RENDER_ONCE) {
      requestingRenderMode.current = NO_REQUEST_RENDER;
    }
  }, [cesium, requestingRenderMode]);

  const explicitRenderRef = useRef<() => void>();

  useEffect(() => {
    explicitRenderRef.current = explicitRender;
  }, [explicitRender]);

  useEffect(() => {
    const viewer = cesium.current?.cesiumElement;
    if (!viewer || viewer.isDestroyed()) return;
    return viewer.scene.postUpdate.addEventListener(() => {
      explicitRenderRef.current?.();
    });
  }, [cesium]);

  // render one frame when scene property changes
  useEffect(() => {
    if (requestingRenderMode) {
      requestingRenderMode.current = REQUEST_RENDER_ONCE;
    }
  }, [property, requestingRenderMode]);

  // force render when timeline is animating or is shouldRender
  useEffect(() => {
    const viewer = cesium.current?.cesiumElement;
    if (!viewer || viewer.isDestroyed()) return;
    if (requestingRenderMode) {
      requestingRenderMode.current =
        isLayerDragging || shouldRender
          ? FORCE_REQUEST_RENDER
          : requestingRenderMode.current === REQUEST_RENDER_ONCE
            ? REQUEST_RENDER_ONCE
            : NO_REQUEST_RENDER;
    }
  }, [cesium, isLayerDragging, shouldRender, requestingRenderMode]);
};
