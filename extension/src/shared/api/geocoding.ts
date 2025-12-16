import { useCallback, useState } from "react";

import type { Address, Area } from "../states/address";
import { usePlateauApiUrl } from "../states/environmentVariables";

export interface GeocodingResponse {
  address: string;
  areas: Area<true>[];
}

export function useGeocodingFetch() {
  const [plateauApiUrl] = usePlateauApiUrl();
  const [data, setData] = useState<GeocodingResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchAreas = useCallback(
    async (params: { longitude: number; latitude: number; includeRadii?: boolean }) => {
      if (!plateauApiUrl) return;

      const url = new URL(`${plateauApiUrl}/geocoding`);
      url.searchParams.set("lon", params.longitude.toString());
      url.searchParams.set("lat", params.latitude.toString());
      if (params.includeRadii) {
        url.searchParams.set("includeRadii", "true");
      }

      setLoading(true);
      setError(null);

      try {
        const response = await fetch(url.toString());
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const result = await response.json();
        setData(result as GeocodingResponse);
        return result as GeocodingResponse;
      } catch (err) {
        setError(err instanceof Error ? err : new Error(String(err)));
        return null;
      } finally {
        setLoading(false);
      }
    },
    [plateauApiUrl],
  );

  return { data, loading, error, fetchAreas };
}

export function useGeocodingLazy(): [
  (params: { longitude: number; latitude: number; includeRadii?: boolean }) => Promise<void>,
  { data: { areas: Address<true> } | null; loading: boolean; error: Error | null },
] {
  const { data, loading, error, fetchAreas } = useGeocodingFetch();

  const fetch = useCallback(
    async (params: { longitude: number; latitude: number; includeRadii?: boolean }) => {
      await fetchAreas(params);
    },
    [fetchAreas],
  );

  return [fetch, { data: data ? { areas: data } : null, loading, error }];
}
