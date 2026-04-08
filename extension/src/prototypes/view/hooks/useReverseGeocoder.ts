import { useState, useCallback, useEffect, useRef } from "react";

import { useGeocodingLazy } from "../../../shared/api/geocoding";
import { useReEarthEvent } from "../../../shared/reearth/hooks";
import { isReEarthAPIv2 } from "../../../shared/reearth/utils/reearth";
import type { Address } from "../../../shared/states/address";

export type ReverseGeocoderResult = Address<true>;

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

  useEffect(() => {
    if (coords.longitude !== undefined && coords.latitude !== undefined) {
      getAreas({
        longitude: coords.longitude,
        latitude: coords.latitude,
        includeRadii: true,
      });
    }
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
