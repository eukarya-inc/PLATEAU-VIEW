import { useEffect } from "react";

import { ReearthEventType, ReEarthV1 } from "../types";
import {
  CameraEventType,
  ReEarthV2,
  SketchEventType,
  ViewerEventType,
} from "../types/reearthPluginAPIv2";
import { SpatialIdEventType } from "../types/reearthPluginAPIv2/spatialId";
import { isReEarthAPIv2 } from "../utils/reearth";

const mouseEvents = {
  click: "click",
  doubleclick: "doubleClick",
  mousedown: "mouseDown",
  mouseup: "mouseUp",
  rightclick: "rightClick",
  rightdown: "rightDown",
  rightup: "rightUp",
  middleclick: "middleClick",
  middledown: "middleDown",
  middleup: "middleUp",
  mousemove: "mouseMove",
  mouseenter: "mouseEnter",
  mouseleave: "mouseLeave",
  wheel: "wheel",
};

const sketchEvents = {
  sketchfeaturecreated: "create",
  sketchtoolchange: "toolChange",
};

const spatialIdEvents = {
  spatialidspacepick: "spacePick",
};

const cameraEvents = {
  cameramove: "move",
};

export const useReEarthEvent = <T extends keyof ReearthEventType>(
  eventName: T,
  cb: (...args: ReearthEventType[T]) => void,
) => {
  useEffect(() => {
    if (isReEarthAPIv2(window.reearth)) {
      if (Object.keys(mouseEvents).includes(eventName)) {
        window.reearth?.viewer?.on?.(
          mouseEvents[eventName as keyof typeof mouseEvents] as keyof ViewerEventType,
          cb as any,
        );
        return () => {
          (window.reearth as ReEarthV2)?.viewer?.off?.(
            mouseEvents[eventName as keyof typeof mouseEvents] as keyof ViewerEventType,
            cb as any,
          );
        };
      } else if (Object.keys(sketchEvents).includes(eventName)) {
        window.reearth?.sketch?.on?.(
          sketchEvents[eventName as keyof typeof sketchEvents] as keyof SketchEventType,
          cb as any,
        );
        return () => {
          (window.reearth as ReEarthV2)?.sketch?.off?.(
            sketchEvents[eventName as keyof typeof sketchEvents] as keyof SketchEventType,
            cb as any,
          );
        };
      } else if (Object.keys(spatialIdEvents).includes(eventName)) {
        window.reearth?.spatialId?.on?.(
          spatialIdEvents[eventName as keyof typeof spatialIdEvents] as keyof SpatialIdEventType,
          cb as any,
        );
        return () => {
          (window.reearth as ReEarthV2)?.spatialId?.off?.(
            spatialIdEvents[eventName as keyof typeof spatialIdEvents] as keyof SpatialIdEventType,
            cb as any,
          );
        };
      } else if (Object.keys(cameraEvents).includes(eventName)) {
        window.reearth?.camera?.on?.(
          cameraEvents[eventName as keyof typeof cameraEvents] as keyof CameraEventType,
          cb as any,
        );
        return () => {
          (window.reearth as ReEarthV2)?.camera?.off?.(
            cameraEvents[eventName as keyof typeof cameraEvents] as keyof CameraEventType,
            cb as any,
          );
        };
      } else {
        console.warn(`Event ${eventName} is not handled`);
      }
    } else {
      window.reearth?.on?.(eventName, cb);
      return () => {
        (window.reearth as ReEarthV1)?.off?.(eventName, cb);
      };
    }
  }, [eventName, cb]);
};
