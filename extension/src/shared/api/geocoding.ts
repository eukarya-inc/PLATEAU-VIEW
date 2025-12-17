import { useCallback, useState } from "react";

import type { Area } from "../states/address";
import { usePlateauApiUrl } from "../states/environmentVariables";

export interface GeocodingResponse {
  address: string;
  areas: Area<true>[];
}

export function useGeocodingLazy(): [
  (params: { longitude: number; latitude: number; includeRadii?: boolean }) => Promise<void>,
  { data: GeocodingResponse | null; loading: boolean },
] {
  const [plateauApiUrl] = usePlateauApiUrl();
  const [data, setData] = useState<GeocodingResponse | null>(null);
  const [loading, setLoading] = useState(false);

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

      try {
        const response = await fetch(url.toString());
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const result = (await response.json()) as GeocodingResponse;
        setData(result);
      } catch (err) {
        console.error("Geocoding error:", err);
        setData(null);
      } finally {
        setLoading(false);
      }
    },
    [plateauApiUrl],
  );

  return [fetchAreas, { data, loading }];
}
