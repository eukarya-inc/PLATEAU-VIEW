import { Cartesian3, Entity, Viewer } from "cesium";
import CesiumDnD, { Context } from "cesium-dnd";
import { RefObject, useCallback, useEffect, useRef } from "react";
import { CesiumComponentRef } from "resium";

import { LatLng } from "../../../utils";
import { isDraggable, isSelectable } from "../common";
import { getTag } from "../Feature";
import { convertCartesian3ToPosition } from "../utils/utils";

export default ({
  cesium,
  isLayerDraggable,
  onLayerDrag,
  onLayerDrop,
}: {
  cesium: RefObject<CesiumComponentRef<Viewer>>;
  isLayerDraggable?: boolean;
  onLayerDrag?: (layerId: string, featureId: string | undefined, position: LatLng) => void;
  onLayerDrop?: (
    layerId: string,
    featureId: string | undefined,
    position: LatLng | undefined,
  ) => void;
}) => {
  // enable Drag and Drop Layers
  const handleLayerDrag = useCallback(
    (e: Entity, position: Cartesian3 | undefined, _context: Context): boolean | void => {
      const viewer = cesium.current?.cesiumElement;
      if (!viewer || viewer.isDestroyed() || !isSelectable(e) || !isDraggable(e)) return false;

      const pos = convertCartesian3ToPosition(cesium.current?.cesiumElement, position);
      if (!pos) return false;

      const tag = getTag(e);
      if (!tag) return false;

      onLayerDrag?.(tag.layerId || "", tag.featureId, pos);
    },
    [cesium, onLayerDrag],
  );

  const handleLayerDrop = useCallback(
    (e: Entity, position: Cartesian3 | undefined): boolean | void => {
      const viewer = cesium.current?.cesiumElement;
      if (!viewer || viewer.isDestroyed()) return false;

      const tag = getTag(e);
      const pos = convertCartesian3ToPosition(cesium.current?.cesiumElement, position);
      onLayerDrop?.(tag?.layerId || "", tag?.featureId || "", pos);

      return false; // let apollo-client handle optimistic updates
    },
    [cesium, onLayerDrop],
  );

  const cesiumDnD = useRef<CesiumDnD>();
  useEffect(() => {
    const viewer = cesium.current?.cesiumElement;
    if (!viewer || viewer.isDestroyed()) return;
    cesiumDnD.current = new CesiumDnD(viewer, {
      onDrag: handleLayerDrag,
      onDrop: handleLayerDrop,
      dragDelay: 1000,
      initialDisabled: !isLayerDraggable,
    });
    return () => {
      if (!viewer || viewer.isDestroyed()) return;
      cesiumDnD.current?.disable();
    };
  }, [cesium, isLayerDraggable, handleLayerDrag, handleLayerDrop]);

  return {
    handleLayerDrag,
    handleLayerDrop,
  };
};
