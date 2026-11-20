import { useCallback, useEffect, useRef, useState } from "react";

import type { Area } from "../states/address";
import { usePlateauApiUrl } from "../states/environmentVariables";

export interface GeocodingResponse {
  address: string;
  areas?: Area<true>[];
}

export function useGeocodingLazy(): [
  (params: { longitude: number; latitude: number; includeRadii?: boolean }) => Promise<void>,
  { data: GeocodingResponse | null; loading: boolean },
] {
  const [plateauApiUrl] = usePlateauApiUrl();
  const [data, setData] = useState<GeocodingResponse | null>(null);
  const [loading, setLoading] = useState(false);
  // Only the latest location matters. Abort the request in flight so that it
  // stops occupying the server and cannot overwrite a newer result.
  const controllerRef = useRef<AbortController>();

  useEffect(() => () => controllerRef.current?.abort(), []);

  const fetchAreas = useCallback(
    async (params: { longitude: number; latitude: number; includeRadii?: boolean }) => {
      if (!plateauApiUrl) return;

      const url = new URL(`${plateauApiUrl}/geocoding`);
      url.searchParams.set("lon", params.longitude.toString());
      url.searchParams.set("lat", params.latitude.toString());
      if (params.includeRadii) {
        url.searchParams.set("includeRadii", "true");
      }

      controllerRef.current?.abort();
      const controller = new AbortController();
      controllerRef.current = controller;

      setLoading(true);

      try {
        const response = await fetch(url.toString(), { signal: controller.signal });
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const result = (await response.json()) as GeocodingResponse;
        if (!controller.signal.aborted) {
          setData(result);
        }
      } catch (err) {
        if (controller.signal.aborted) return;
        console.error("Geocoding error:", err);
        setData(null);
      } finally {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      }
    },
    [plateauApiUrl],
  );

  return [fetchAreas, { data, loading }];
}

export function useGeocoding({
  longitude,
  latitude,
  includeRadii,
}: {
  longitude: number;
  latitude: number;
  includeRadii?: boolean;
}): { data: GeocodingResponse | null; loading: boolean } {
  const [fetchAreas, result] = useGeocodingLazy();
  useEffect(() => {
    void fetchAreas({ longitude, latitude, includeRadii });
  }, [fetchAreas, longitude, latitude, includeRadii]);
  return result;
}
