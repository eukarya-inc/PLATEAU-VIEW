import { Cartesian3, Color, DirectionalLight, SceneMode, SunLight, Viewer } from "cesium";
import { RefObject, useMemo } from "react";
import { CesiumComponentRef } from "resium";

import { ViewerProperty } from "../..";

// TODO: move all viewer property -> resium component prop logic here (from the gerneal long Cesium/hooks file)
export default ({
  cesium,
  property,
}: {
  cesium: RefObject<CesiumComponentRef<Viewer>>;
  property?: ViewerProperty;
}) => {
  const sceneLight = useMemo(() => {
    let light;
    if (property?.scene?.light?.type === "sunLight") {
      light = new SunLight({
        color: property.scene?.light?.color
          ? Color.fromCssColorString(property.scene.light.color)
          : undefined,
        intensity: property.scene?.light?.intensity,
      });
    } else if (property?.scene?.light?.type === "directionalLight") {
      light = new DirectionalLight({
        direction: new Cartesian3(
          property?.scene?.light?.direction?.[0] ?? 1,
          property?.scene?.light?.direction?.[1] ?? 0,
          property?.scene?.light?.direction?.[2] ?? 0,
        ),
        color: property.scene?.light?.color
          ? Color.fromCssColorString(property.scene.light.color)
          : undefined,
        intensity: property.scene?.light?.intensity,
      });
    } else {
      light = cesium.current?.cesiumElement?.scene.light;
      if (light) {
        light.color = property?.scene?.light?.color
          ? Color.fromCssColorString(property.scene.light.color)
          : light.color;
        light.intensity = property?.scene?.light?.intensity
          ? property.scene.light.intensity
          : light.intensity;
      }
    }
    return light;
  }, [
    cesium,
    property?.scene?.light?.type,
    property?.scene?.light?.color,
    property?.scene?.light?.direction,
    property?.scene?.light?.intensity,
  ]);

  const sceneBackgroundColor = useMemo(
    () =>
      property?.scene?.backgroundColor
        ? Color.fromCssColorString(property.scene.backgroundColor)
        : undefined,
    [property?.scene?.backgroundColor],
  );

  const sceneMsaaSamples = useMemo(() => {
    // TODO: FXAA doesn't support alpha blending in Cesium, so we will enable FXAA when this is fixed.
    // viewer.scene.postProcessStages.fxaa.enabled = property?.render?.antialias === "high";
    return property?.render?.antialias === "extreme"
      ? 8
      : property?.render?.antialias === "high"
        ? 6
        : property?.render?.antialias === "medium"
          ? 4
          : 1;
  }, [property?.render?.antialias]);

  const sceneMode = useMemo(() => {
    return property?.scene?.mode === "2d"
      ? SceneMode.SCENE2D
      : property?.scene?.mode === "columbus"
        ? SceneMode.COLUMBUS_VIEW
        : SceneMode.SCENE3D;
  }, [property?.scene?.mode]);

  return {
    sceneLight,
    sceneBackgroundColor,
    sceneMsaaSamples,
    sceneMode,
  };
};
