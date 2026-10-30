import { type MultiPolygon, type Polygon } from "geojson";
import { useCallback, useEffect, useRef, useState } from "react";

import { usePlateauApiUrl } from "../states/environmentVariables";

export interface EstatArea {
  id: string;
  prefectureCode: string;
  municipalityCode: string;
  name: string;
  address: string;
  addressComponents: string[];
  bbox: number[];
}

export interface EstatAreaGeometry {
  id: string;
  geometry: Polygon | MultiPolygon;
}

export function useEstatAreasLazy(): [
  (searchTokens: string[]) => Promise<void>,
  { data: EstatArea[] | undefined; loading: boolean },
] {
  const [plateauApiUrl] = usePlateauApiUrl();
  const [data, setData] = useState<EstatArea[]>();
  const [loading, setLoading] = useState(false);
  // Searches are debounced but can still resolve out of order; only the
  // latest request may update the state.
  const latestRequest = useRef(0);

  const fetchAreas = useCallback(
    async (searchTokens: string[]) => {
      if (!plateauApiUrl) return;

      const url = new URL(`${plateauApiUrl}/estat/areas`);
      for (const token of searchTokens) {
        url.searchParams.append("q", token);
      }

      const request = ++latestRequest.current;
      setLoading(true);
      try {
        const response = await fetch(url.toString());
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const result = (await response.json()) as EstatArea[];
        if (request === latestRequest.current) {
          setData(result);
        }
      } catch (err) {
        console.error("e-Stat area search error:", err);
        if (request === latestRequest.current) {
          setData(undefined);
        }
      } finally {
        if (request === latestRequest.current) {
          setLoading(false);
        }
      }
    },
    [plateauApiUrl],
  );

  return [fetchAreas, { data, loading }];
}

export function useEstatAreaGeometry({ areaId }: { areaId: string }): {
  data: EstatAreaGeometry | null;
  loading: boolean;
} {
  const [plateauApiUrl] = usePlateauApiUrl();
  const [data, setData] = useState<EstatAreaGeometry | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!plateauApiUrl) return;

    const controller = new AbortController();
    setLoading(true);
    fetch(`${plateauApiUrl}/estat/areas/${encodeURIComponent(areaId)}/geometry`, {
      signal: controller.signal,
    })
      .then(async response => {
        if (response.status === 404) return null;
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        return (await response.json()) as EstatAreaGeometry;
      })
      .then(result => {
        setData(result);
        setLoading(false);
      })
      .catch(err => {
        if (controller.signal.aborted) return;
        console.error("e-Stat area geometry error:", err);
        setData(null);
        setLoading(false);
      });
    return () => {
      controller.abort();
    };
  }, [plateauApiUrl, areaId]);

  return { data, loading };
}
