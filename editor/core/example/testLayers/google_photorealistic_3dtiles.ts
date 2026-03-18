import { Layer } from "@reearth/core";

export const GOOGLE_PHOTOREALISTIC_3DTILES: Layer = {
  id: "google_photorealistic_3dtiles",
  type: "simple",
  data: {
    type: "google-photorealistic",
    serviceTokens: {
      googleMapApiKey: import.meta.env.EXAMPLE_GOOGLE_MAP_API_KEY || "",
    },
  },
  "3dtiles": {},
};
