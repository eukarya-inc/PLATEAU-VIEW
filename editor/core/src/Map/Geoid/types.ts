export type GeoidServer = {
  url: string; // URL of the geoid server. use ${lat} ${lng} for lat/lng placeholders. Example: "https://mock.com/api/altitude?lat=${lat}&lng=${lng}"
  geoidProperty: string; // TODO: support json path
};

export type GeoidRef = {
  getGeoidHeight: (lat?: number, lng?: number) => Promise<number | undefined>;
};
