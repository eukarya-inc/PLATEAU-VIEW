// TODO: Refactor: move cesium related code to engine.
import invariant from "tiny-invariant";
import { createMachine, type StateFrom } from "xstate";

import { type SketchType } from "./types";

export type Position2d = [number, number];
export type Position3d = [number, number, number];

export type EventObject =
  | ((
      | { type: "MARKER" }
      | { type: "POLYLINE" }
      | { type: "CIRCLE" }
      | { type: "RECTANGLE" }
      | { type: "POLYGON" }
      | { type: "EXTRUDED_CIRCLE" }
      | { type: "EXTRUDED_RECTANGLE" }
      | { type: "EXTRUDED_POLYGON" }
      | { type: "NEXT" }
      | { type: "EXTRUDE" }
    ) & {
      pointerPosition: Position2d;
      controlPoint: Position3d;
    })
  | ((
      | { type: "EDIT_MARKER" }
      | { type: "EDIT_POLYLINE" }
      | { type: "EDIT_CIRCLE" }
      | { type: "EDIT_RECTANGLE" }
      | { type: "EDIT_POLYGON" }
      | { type: "EDIT_EXTRUDED_CIRCLE" }
      | { type: "EDIT_EXTRUDED_RECTANGLE" }
      | { type: "EDIT_EXTRUDED_POLYGON" }
      | { type: "CATCH" }
      | { type: "UPDATE" }
      | { type: "MOVE" }
      | { type: "RELEASE" }
    ) & {
      extrudedHeight?: number;
      controlPoints: Position3d[];
      catchedControlPointIndex?: number;
      catchedExtrudedPoint?: boolean;
    })
  | { type: "CREATE" }
  | { type: "CANCEL" }
  | { type: "ABORT" }
  | { type: "EXIT_EDIT" };

export interface Context {
  lastPointerPosition?: Position2d;
  lastControlPoint?: Position3d;
  type?: SketchType;
  controlPoints?: Position3d[];
  catchedControlPointIndex?: number;
  catchedExtrudedPoint?: boolean;
  originalControlPoint?: Position3d;
}

export function createSketchMachine() {
  return createMachine(
    {
      id: "sketch",
      initial: "idle",
      context: {} as Context,
      states: {
        idle: {
          on: {
            MARKER: {
              target: "drawing.marker",
              actions: ["createMarker"],
            },
            POLYLINE: {
              target: "drawing.polyline",
              actions: ["createPolyline"],
            },
            CIRCLE: {
              target: "drawing.circle",
              actions: ["createCircle"],
            },
            RECTANGLE: {
              target: "drawing.rectangle",
              actions: ["createRectangle"],
            },
            POLYGON: {
              target: "drawing.polygon",
              actions: ["createPolygon"],
            },
            EXTRUDED_CIRCLE: {
              target: "drawing.circle",
              actions: ["createExtrudedCircle"],
            },
            EXTRUDED_RECTANGLE: {
              target: "drawing.extrudedRectangle",
              actions: ["createExtrudedRectangle"],
            },
            EXTRUDED_POLYGON: {
              target: "drawing.extrudedPolygon",
              actions: ["createExtrudedPolygon"],
            },
            EDIT_MARKER: {
              target: "editing.marker",
              actions: ["editMarker"],
            },
            EDIT_POLYLINE: {
              target: "editing.polyline",
              actions: ["editPolyline"],
            },
            EDIT_CIRCLE: {
              target: "editing.circle",
              actions: ["editCircle"],
            },
            EDIT_RECTANGLE: {
              target: "editing.rectangle",
              actions: ["editRectangle"],
            },
            EDIT_POLYGON: {
              target: "editing.polygon",
              actions: ["editPolygon"],
            },
            EDIT_EXTRUDED_CIRCLE: {
              target: "editing.circle",
              actions: ["editExtrudedCircle"],
            },
            EDIT_EXTRUDED_RECTANGLE: {
              target: "editing.extrudedRectangle",
              actions: ["editExtrudedRectangle"],
            },
            EDIT_EXTRUDED_POLYGON: {
              target: "editing.extrudedPolygon",
              actions: ["editExtrudedPolygon"],
            },
          },
        },
        drawing: {
          states: {
            marker: {
              initial: "vertex",
              states: {
                vertex: {},
              },
            },
            polyline: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: {
                      target: "vertex",
                      internal: true,
                      actions: ["pushPosition"],
                    },
                  },
                },
              },
            },
            circle: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: {
                      target: "#sketch.extruding",
                      actions: ["pushPosition"],
                    },
                  },
                },
              },
            },
            rectangle: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: [
                      {
                        target: "vertex",
                        internal: true,
                        actions: ["pushPosition"],
                      },
                    ],
                  },
                },
              },
            },
            extrudedRectangle: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: [
                      {
                        target: "#sketch.extruding",
                        cond: "willRectangleComplete",
                        actions: ["pushPosition", "recordOriginalControlPoint"],
                      },
                      {
                        target: "vertex",
                        internal: true,
                        actions: ["pushPosition"],
                      },
                    ],
                  },
                },
              },
            },
            polygon: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: {
                      target: "vertex",
                      internal: true,
                      actions: ["pushPosition"],
                    },
                  },
                },
              },
            },
            extrudedPolygon: {
              initial: "vertex",
              states: {
                vertex: {
                  on: {
                    NEXT: {
                      target: "vertex",
                      internal: true,
                      actions: ["pushPosition"],
                    },
                    EXTRUDE: {
                      target: "#sketch.extruding",
                      actions: ["pushPosition"],
                    },
                  },
                },
              },
            },
            history: {
              type: "history",
            },
          },
          on: {
            CANCEL: [
              {
                target: ".history",
                cond: "canPopPosition",
                actions: ["popPosition"],
              },
              {
                target: "idle",
                actions: ["clearDrawing"],
              },
            ],
            ABORT: {
              target: "idle",
              actions: ["clearDrawing"],
            },
            CREATE: {
              target: "idle",
              actions: ["clearDrawing"],
            },
          },
        },
        extruding: {
          on: {
            CREATE: {
              target: "idle",
              actions: ["clearDrawing"],
            },
            CANCEL: {
              target: "drawing.history",
              actions: ["popPosition"],
            },
            ABORT: {
              target: "idle",
              actions: ["clearDrawing"],
            },
          },
        },
        editing: {
          states: {
            marker: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            polyline: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                    UPDATE: {
                      target: "waiting",
                      internal: true,
                      actions: ["updateControlPoints"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            circle: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            rectangle: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            polygon: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                    UPDATE: {
                      target: "waiting",
                      internal: true,
                      actions: ["updateControlPoints"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            extrudedCircle: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            extrudedRectangle: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
            extrudedPolygon: {
              initial: "waiting",
              states: {
                waiting: {
                  on: {
                    CATCH: {
                      target: "moving",
                      internal: true,
                      actions: ["catchControlPoint"],
                    },
                    UPDATE: {
                      target: "waiting",
                      internal: true,
                      actions: ["updateControlPoints"],
                    },
                  },
                },
                moving: {
                  on: {
                    MOVE: {
                      target: "moving",
                      internal: true,
                      actions: ["moveControlPoint"],
                    },
                    RELEASE: {
                      target: "waiting",
                      actions: ["releaseControlPoint"],
                    },
                  },
                },
              },
            },
          },
          on: {
            EXIT_EDIT: {
              target: "idle",
              actions: ["clearDrawing"],
            },
          },
        },
      },
      schema: {
        events: {} as unknown as EventObject,
      },
      predictableActionArguments: true,
      preserveActionOrder: true,
      tsTypes: {} as import("./sketchMachine.typegen").Typegen0,
    },
    {
      guards: {
        canPopPosition: context => {
          return context.controlPoints != null && context.controlPoints.length > 1;
        },
        willRectangleComplete: context => {
          return context.controlPoints != null && context.controlPoints.length === 2;
        },
      },
      actions: {
        createMarker: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "marker";
          context.controlPoints = [controlPoint];
        },
        editMarker: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "marker";
          context.controlPoints = event.controlPoints;
        },
        createPolyline: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "polyline";
          context.controlPoints = [controlPoint];
        },
        editPolyline: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "polyline";
          context.controlPoints = event.controlPoints;
        },
        createCircle: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "circle";
          context.controlPoints = [controlPoint];
        },
        editCircle: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "circle";
          context.controlPoints = event.controlPoints;
        },
        createRectangle: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "rectangle";
          context.controlPoints = [controlPoint];
        },
        editRectangle: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "rectangle";
          context.controlPoints = event.controlPoints;
        },
        createPolygon: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "polygon";
          context.controlPoints = [controlPoint];
        },
        editPolygon: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "polygon";
          context.controlPoints = event.controlPoints;
        },
        createExtrudedCircle: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "extrudedCircle";
          context.controlPoints = [controlPoint];
        },
        editExtrudedCircle: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "extrudedCircle";
          context.controlPoints = event.controlPoints;
        },
        createExtrudedRectangle: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "extrudedRectangle";
          context.controlPoints = [controlPoint];
        },
        editExtrudedRectangle: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "extrudedRectangle";
          context.controlPoints = event.controlPoints;
        },
        createExtrudedPolygon: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.type = "extrudedPolygon";
          context.controlPoints = [controlPoint];
        },
        editExtrudedPolygon: (context, event) => {
          context.lastControlPoint = undefined;
          context.type = "extrudedPolygon";
          context.controlPoints = event.controlPoints;
        },
        pushPosition: (context, event) => {
          context.lastPointerPosition = [...event.pointerPosition];
          const controlPoint = [...event.controlPoint] as Position3d;
          context.lastControlPoint = controlPoint;
          context.controlPoints?.push(controlPoint);
        },
        popPosition: context => {
          invariant(context.controlPoints != null);
          invariant(context.controlPoints.length > 1);
          context.controlPoints.pop();
        },
        clearDrawing: context => {
          delete context.lastControlPoint;
          delete context.type;
          delete context.controlPoints;
          delete context.catchedControlPointIndex;
          delete context.catchedExtrudedPoint;
          delete context.originalControlPoint;
        },
        catchControlPoint: (context, event) => {
          context.catchedControlPointIndex = event.catchedControlPointIndex;
          context.catchedExtrudedPoint = event.catchedExtrudedPoint;
        },
        moveControlPoint: (context, event) => {
          context.controlPoints = event.controlPoints;
        },
        releaseControlPoint: context => {
          delete context.catchedControlPointIndex;
          delete context.catchedExtrudedPoint;
        },
        updateControlPoints: (context, event) => {
          context.controlPoints = event.controlPoints;
        },
        recordOriginalControlPoint: (context, event) => {
          context.originalControlPoint = [...event.controlPoint] as Position3d;
        },
      },
    },
  );
}

export type SketchMachine = ReturnType<typeof createSketchMachine>;
export type SketchMachineState = StateFrom<SketchMachine>;
