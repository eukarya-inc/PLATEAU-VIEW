import { useCallback } from "react";

import { isReEarthAPIv2 } from "../utils/reearth";

export const useViewer = () => {
  const getGeoidHeight = useCallback((lng?: number, lat?: number) => {
    if (isReEarthAPIv2(window.reearth)) {
      return window.reearth?.viewer?.tools?.getGeoidHeight(lng, lat);
    }
  }, []);

  return {
    getGeoidHeight,
  };
};
