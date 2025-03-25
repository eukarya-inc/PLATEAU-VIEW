import { useCallback } from "react";

import { SpatialIdPickSpaceOptions } from "../types/reearthPluginAPIv2/spatialId";
import { isReEarthAPIv2 } from "../utils/reearth";

export const useSpatialId = () => {
  const handlePickSpace = useCallback((options?: SpatialIdPickSpaceOptions) => {
    if (isReEarthAPIv2(window.reearth)) {
      window.reearth?.spatialId?.pickSpace?.(options);
    }
  }, []);

  const handleExitPickSpace = useCallback(() => {
    if (isReEarthAPIv2(window.reearth)) {
      window.reearth?.spatialId?.exitPickSpace?.();
    }
  }, []);

  return {
    handlePickSpace,
    handleExitPickSpace,
  };
};
