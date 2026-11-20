import { useState, useCallback, useEffect, useRef } from "react";

import { useGeocodingLazy } from "../../../shared/api/geocoding";
import { useReEarthEvent } from "../../../shared/reearth/hooks";
import { isReEarthAPIv2 } from "../../../shared/reearth/utils/reearth";
import type { Address } from "../../../shared/states/address";

export type ReverseGeocoderResult = Address<true>;

const GEOCODING_DEBOUNCE_MS = 300;

export function useReverseGeocoder(): ReverseGeocoderResult | undefined {
  const [coords, setCoords] = useState<{
    longitude?: number;
    latitude?: number;
  }>({});

  const viewSize = useRef<number>();

  const [getAreas, { data }] = useGeocodingLazy();
  const [result, setResult] = useState<ReverseGeocoderResult>();

  useEffect(() => {
    if (data) {
      const areas = { ...data };
      if (viewSize.current) {
        const threshold = viewSize.current * 0.5;
        areas.areas = areas.areas?.filter(area => area.radius > threshold) ?? [];
      }
      setResult(areas as ReverseGeocoderResult);
    }
  }, [data]);

  // "cameramove" fires on every frame while the camera moves, so wait until
  // the camera settles instead of requesting an address for each frame.
  useEffect(() => {
    const { longitude, latitude } = coords;
    if (longitude === undefined || latitude === undefined) return;
    const timer = window.setTimeout(() => {
      void getAreas({ longitude, latitude, includeRadii: true });
    }, GEOCODING_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [coords, getAreas]);

  const updateFovInfo = useCallback(() => {
    const fovInfo = isReEarthAPIv2(window.reearth)
      ? window.reearth?.camera?.getGlobeIntersection({ withTerrain: true, calcViewSize: true })
      : window.reearth?.camera?.getFovInfo({ withTerrain: true, calcViewSize: true });
    setCoords({
      longitude: fovInfo?.center?.lng,
      latitude: fovInfo?.center?.lat,
    });
    viewSize.current = fovInfo?.viewSize;
  }, []);

  useReEarthEvent("cameramove", updateFovInfo);

  return result;
}
