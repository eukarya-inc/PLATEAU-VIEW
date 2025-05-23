import { Viewer } from "cesium";

declare global {
  interface Window {
    REEARTH_E2E_ACCESS_TOKEN?: string;
    REEARTH_E2E_CESIUM_VIEWER?: any;
  }
}

export function e2eAccessToken(): string | undefined {
  return window.REEARTH_E2E_ACCESS_TOKEN;
}

export function setE2ECesiumViewer(viewer: Viewer | undefined) {
  if (viewer) {
    window.REEARTH_E2E_CESIUM_VIEWER = viewer;
  } else {
    delete window.REEARTH_E2E_CESIUM_VIEWER;
  }
}
