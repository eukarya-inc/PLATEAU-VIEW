import { Math as CesiumMath, Scene, Viewer } from "cesium";
import { isEqual } from "lodash-es";
import { useCallback, useEffect, useRef } from "react";
import { CesiumComponentRef } from "resium";
import { RefObject } from "use-callback-ref/dist/es5/types";
import { useCustomCompareCallback } from "use-custom-compare";

import { EngineRef, ViewerProperty } from "../..";
import { Camera } from "../../../mantle";
import { FEATURE_FLAGS } from "../../../Visualizer";
import { getCamera } from "../common";

import { useCameraLimiter } from "./useCameraLimiter";

export default ({
  cesium,
  property,
  camera,
  featureFlags,
  engineAPI,
  cameraForceHorizontalRoll = false,
  onCameraChange,
}: {
  cesium: RefObject<CesiumComponentRef<Viewer>>;
  property?: ViewerProperty;
  engineAPI: EngineRef;
  featureFlags: number;
  camera?: Camera;
  cameraForceHorizontalRoll?: boolean;
  onCameraChange?: (camera: Camera) => void;
}) => {
  // cache the camera data emitted from viewer camera change
  const emittedCamera = useRef<Camera[]>([]);
  const updateCamera = useCallback(() => {
    const viewer = cesium?.current?.cesiumElement;
    if (!viewer || viewer.isDestroyed() || !onCameraChange) return;

    const c = getCamera(viewer);
    if (c && !isEqual(c, camera)) {
      emittedCamera.current.push(c);
      // The state change is not sync now. This number is how many state updates can actually happen to be merged within one re-render.
      if (emittedCamera.current.length > 10) {
        emittedCamera.current.shift();
      }
      onCameraChange?.(c);
    }
  }, [cesium, camera, onCameraChange]);

  const handleCameraChange = useCallback(() => {
    updateCamera();
  }, [updateCamera]);

  const handleCameraMoveEnd = useCallback(() => {
    updateCamera();
  }, [updateCamera]);

  useEffect(() => {
    if (camera && !emittedCamera.current.includes(camera)) {
      engineAPI.flyTo(camera, { duration: 0 });
      emittedCamera.current = [];
    }
  }, [camera, engineAPI]);

  useEffect(() => {
    if (!cesium.current?.cesiumElement) return;
    const allowCameraMove = !!(featureFlags & FEATURE_FLAGS.CAMERA_MOVE);
    const allowCameraZoom = !!(featureFlags & FEATURE_FLAGS.CAMERA_ZOOM);
    const allowCameraTilt = !!(featureFlags & FEATURE_FLAGS.CAMERA_TILT);
    const allowCameraLook = !!(featureFlags & FEATURE_FLAGS.CAMERA_LOOK);
    cesium.current.cesiumElement.scene.screenSpaceCameraController.enableTranslate =
      allowCameraMove;
    cesium.current.cesiumElement.scene.screenSpaceCameraController.enableRotate = allowCameraMove;
    cesium.current.cesiumElement.scene.screenSpaceCameraController.enableLook = allowCameraLook;
    cesium.current.cesiumElement.scene.screenSpaceCameraController.enableTilt = allowCameraTilt;
    cesium.current.cesiumElement.scene.screenSpaceCameraController.enableZoom = allowCameraZoom;
  }, [cesium, featureFlags]);

  // move to initial position at startup
  const initialCameraFlight = useRef(false);

  const mountCamera = useCustomCompareCallback(
    () => {
      if (initialCameraFlight.current) return;
      initialCameraFlight.current = true;
      if (property?.camera?.limiter?.enabled && property?.camera?.limiter?.targetArea) {
        engineAPI.flyTo(property?.camera?.limiter?.targetArea, { duration: 0 });
      } else if (camera) {
        engineAPI.flyTo(camera, { duration: 0 });
      }
    },
    [engineAPI, property?.camera?.limiter?.enabled],
    (prevDeps, nextDeps) => prevDeps[0] === nextDeps[0] && prevDeps[1] === nextDeps[1],
  );

  const unmountCamera = useCallback(() => {
    initialCameraFlight.current = false;
  }, []);

  // camera limiter
  const { cameraViewBoundaries, cameraViewOuterBoundaries, cameraViewBoundariesMaterial } =
    useCameraLimiter(cesium, camera, property?.camera?.limiter);

  // horizontal roll
  const fixCameraHorizontal = useCallback(
    (scene: Scene) => {
      if (!scene.camera || !cameraForceHorizontalRoll) return;
      if (Math.abs(CesiumMath.negativePiToPi(scene.camera.roll)) > Math.PI / 86400) {
        scene.camera.setView({
          orientation: {
            heading: scene.camera.heading,
            pitch: scene.camera.pitch,
            roll: 0,
          },
        });
      }
    },
    [cameraForceHorizontalRoll],
  );

  useEffect(() => {
    const viewer = cesium.current?.cesiumElement;
    if (!viewer) return;
    return viewer.scene.preRender.addEventListener(fixCameraHorizontal);
  }, [cesium, fixCameraHorizontal]);

  return {
    cameraViewBoundaries,
    cameraViewOuterBoundaries,
    cameraViewBoundariesMaterial,
    mountCamera,
    unmountCamera,
    handleCameraChange,
    handleCameraMoveEnd,
  };
};
