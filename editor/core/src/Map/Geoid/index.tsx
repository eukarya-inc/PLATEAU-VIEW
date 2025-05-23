import {
  forwardRef,
  ForwardRefRenderFunction,
  useCallback,
  useImperativeHandle,
  useRef,
} from "react";

import { GeoidServer, GeoidRef } from "./types";

export type GeoidProps = {
  geoidServer?: GeoidServer;
};

const MAX_GEOID_CACHE_SIZE = 1000;

const Geoid: ForwardRefRenderFunction<GeoidRef, GeoidProps> = ({ geoidServer }, ref) => {
  const geoidCache = useRef<Map<string, number>>(new Map());

  const getGeoidHeight = useCallback(
    async (lng?: number, lat?: number): Promise<number | undefined> => {
      if (!geoidServer?.url || !geoidServer?.geoidProperty) {
        console.error("Geoid: Server is not set properly");
        return Promise.resolve(undefined);
      }

      if (lat === undefined || lng === undefined) {
        console.error("Geoid: Invalid lat or lng");
        return Promise.resolve(undefined);
      }

      const cache = geoidCache.current.get(`${lng},${lat}`);
      if (cache) return Promise.resolve(cache);

      return fetch(
        geoidServer.url.replace("${lng}", lng.toString()).replace("${lat}", lat.toString()),
      )
        .then(res => {
          return res.json().then((result: any) => {
            if (!result) return undefined;
            const geoid = Number(result[geoidServer?.geoidProperty]);
            if (!isNaN(geoid)) {
              const cache = geoidCache.current;
              if (cache.size > MAX_GEOID_CACHE_SIZE) {
                const firstKey = cache.keys().next().value;
                if (firstKey) cache.delete(firstKey);
              }
              cache.set(`${lng},${lat}`, geoid);
              return geoid;
            }
            return undefined;
          });
        })
        .catch(e => {
          console.error("Failed to fetch geoid height", e);
          return undefined;
        });
    },
    [geoidServer?.url, geoidServer?.geoidProperty],
  );

  useImperativeHandle(ref, () => ({
    getGeoidHeight,
  }));
  return null;
};

export default forwardRef(Geoid);
