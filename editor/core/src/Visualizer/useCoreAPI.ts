import { useEffect, useMemo, useRef } from "react";

import { version } from "../../package.json";
import { ViewerProperty } from "../engines";

interface CoreAPI {
  readonly version: string;
  readonly viewerProperty?: ViewerProperty;
}

export default ({ viewerProperty }: { viewerProperty?: ViewerProperty }) => {
  const viewerPropertyRef = useRef(viewerProperty);
  viewerPropertyRef.current = viewerProperty;

  const coreAPI: CoreAPI = useMemo(
    () => ({
      get version() {
        return version;
      },
      get viewerProperty() {
        return viewerPropertyRef.current;
      },
    }),
    [],
  );

  useEffect(() => {
    (window as any).reearth_core = coreAPI;
  }, [coreAPI]);
};
