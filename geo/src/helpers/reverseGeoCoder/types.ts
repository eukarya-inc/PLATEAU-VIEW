// Per-request timeout (ms) for the external reverse-geocoder upstreams. axios
// defaults to no timeout, so a hung upstream would otherwise block the resolver
// and hold a socket indefinitely.
export const UPSTREAM_TIMEOUT_MS = 30_000;

export type Areas = {
  municipalityCode?: string;
  name?: string;
};

export type AreasFetcherBase = (
  url: string,
  lon: number,
  lat: number,
) => Promise<Areas | undefined>;
export type AreasFetcher = (lon: number, lat: number) => Promise<Areas | undefined>;
