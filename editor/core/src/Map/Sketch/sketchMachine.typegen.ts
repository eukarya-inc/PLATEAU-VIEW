// This file was automatically generated. Edits will be overwritten

export interface Typegen0 {
  "@@xstate/typegen": true;
  internalEvents: {
    "xstate.init": { type: "xstate.init" };
  };
  invokeSrcNameMap: {};
  missingImplementations: {
    actions: never;
    delays: never;
    guards: never;
    services: never;
  };
  eventsCausingActions: {
    catchControlPoint: "CATCH";
    clearDrawing: "ABORT" | "CANCEL" | "CREATE" | "EXIT_EDIT";
    createCircle: "CIRCLE";
    createExtrudedCircle: "EXTRUDED_CIRCLE";
    createExtrudedPolygon: "EXTRUDED_POLYGON";
    createExtrudedRectangle: "EXTRUDED_RECTANGLE";
    createMarker: "MARKER";
    createPolygon: "POLYGON";
    createPolyline: "POLYLINE";
    createRectangle: "RECTANGLE";
    editCircle: "EDIT_CIRCLE";
    editExtrudedCircle: "EDIT_EXTRUDED_CIRCLE";
    editExtrudedPolygon: "EDIT_EXTRUDED_POLYGON";
    editExtrudedRectangle: "EDIT_EXTRUDED_RECTANGLE";
    editMarker: "EDIT_MARKER";
    editPolygon: "EDIT_POLYGON";
    editPolyline: "EDIT_POLYLINE";
    editRectangle: "EDIT_RECTANGLE";
    moveControlPoint: "MOVE";
    popPosition: "CANCEL";
    pushPosition: "EXTRUDE" | "NEXT";
    recordOriginalControlPoint: "NEXT";
    releaseControlPoint: "RELEASE";
    updateControlPoints: "UPDATE";
  };
  eventsCausingDelays: {};
  eventsCausingGuards: {
    canPopPosition: "CANCEL";
    willRectangleComplete: "NEXT";
  };
  eventsCausingServices: {};
  matchesStates:
    | "drawing"
    | "drawing.circle"
    | "drawing.circle.vertex"
    | "drawing.extrudedPolygon"
    | "drawing.extrudedPolygon.vertex"
    | "drawing.extrudedRectangle"
    | "drawing.extrudedRectangle.vertex"
    | "drawing.marker"
    | "drawing.marker.vertex"
    | "drawing.polygon"
    | "drawing.polygon.vertex"
    | "drawing.polyline"
    | "drawing.polyline.vertex"
    | "drawing.rectangle"
    | "drawing.rectangle.vertex"
    | "editing"
    | "editing.circle"
    | "editing.circle.moving"
    | "editing.circle.waiting"
    | "editing.extrudedCircle"
    | "editing.extrudedCircle.moving"
    | "editing.extrudedCircle.waiting"
    | "editing.extrudedPolygon"
    | "editing.extrudedPolygon.moving"
    | "editing.extrudedPolygon.waiting"
    | "editing.extrudedRectangle"
    | "editing.extrudedRectangle.moving"
    | "editing.extrudedRectangle.waiting"
    | "editing.marker"
    | "editing.marker.moving"
    | "editing.marker.waiting"
    | "editing.polygon"
    | "editing.polygon.moving"
    | "editing.polygon.waiting"
    | "editing.polyline"
    | "editing.polyline.moving"
    | "editing.polyline.waiting"
    | "editing.rectangle"
    | "editing.rectangle.moving"
    | "editing.rectangle.waiting"
    | "extruding"
    | "idle"
    | {
        drawing?:
          | "circle"
          | "extrudedPolygon"
          | "extrudedRectangle"
          | "marker"
          | "polygon"
          | "polyline"
          | "rectangle"
          | {
              circle?: "vertex";
              extrudedPolygon?: "vertex";
              extrudedRectangle?: "vertex";
              marker?: "vertex";
              polygon?: "vertex";
              polyline?: "vertex";
              rectangle?: "vertex";
            };
        editing?:
          | "circle"
          | "extrudedCircle"
          | "extrudedPolygon"
          | "extrudedRectangle"
          | "marker"
          | "polygon"
          | "polyline"
          | "rectangle"
          | {
              circle?: "moving" | "waiting";
              extrudedCircle?: "moving" | "waiting";
              extrudedPolygon?: "moving" | "waiting";
              extrudedRectangle?: "moving" | "waiting";
              marker?: "moving" | "waiting";
              polygon?: "moving" | "waiting";
              polyline?: "moving" | "waiting";
              rectangle?: "moving" | "waiting";
            };
      };
  tags: never;
}
