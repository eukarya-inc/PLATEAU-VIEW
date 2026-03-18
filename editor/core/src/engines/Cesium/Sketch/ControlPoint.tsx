import {
  CallbackProperty,
  HeightReference,
  type Cartesian3,
  type PositionProperty,
  type Property,
} from "@cesium/engine";
import { useCallback, useMemo, useRef, type FC } from "react";
import { Entity } from "resium";
import invariant from "tiny-invariant";

import { useConstant } from "../../../utils";

import { ADDING_POINT_COLOR, DEFAULT_EDIT_COLOR, SELECTED_EDIT_COLOR } from "./constants";

import { ControlPointMouseEventHandler } from ".";

let drawPointImage: HTMLCanvasElement | undefined;
let editPointImage: HTMLCanvasElement | undefined;
let selectedPointImage: HTMLCanvasElement | undefined;
let addingPointImage: HTMLCanvasElement | undefined;

export interface ControlPointProps {
  position: Property | Cartesian3;
  clampToGround?: boolean;
  index: number;
  isEditing?: boolean;
  isSelected?: boolean;
  isAddingPoint?: boolean;
  isExtrudedControlPoint?: boolean;
  handleControlPointMouseEvent?: ControlPointMouseEventHandler;
}

export const ControlPoint: FC<ControlPointProps> = ({
  position,
  clampToGround = false,
  index,
  isEditing,
  isSelected,
  isAddingPoint,
  isExtrudedControlPoint,
  handleControlPointMouseEvent,
}) => {
  const positionRef = useRef(position);
  positionRef.current = position;
  const positionProperty = useConstant(
    () =>
      new CallbackProperty(
        (time, result) =>
          "getValue" in positionRef.current
            ? positionRef.current.getValue(time, result)
            : positionRef.current,
        false,
      ) as unknown as PositionProperty,
  );

  const options = useMemo(
    () => ({
      position: positionProperty,
      billboard: {
        image: isEditing
          ? isAddingPoint
            ? getAddingPointImage()
            : isSelected
              ? getSelectedPointImage()
              : getEditPointImage()
          : getDrawPointImage(),
        width: isEditing ? 16 : 8,
        height: isEditing ? 16 : 8,
        heightReference: clampToGround ? HeightReference.CLAMP_TO_GROUND : HeightReference.NONE,
        disableDepthTestDistance: Infinity,
      },
    }),
    [clampToGround, positionProperty, isAddingPoint, isEditing, isSelected],
  );

  const handleMouseDown = useCallback(() => {
    handleControlPointMouseEvent?.(index, !!isExtrudedControlPoint, "mousedown");
  }, [index, isExtrudedControlPoint, handleControlPointMouseEvent]);

  const handleMouseClick = useCallback(() => {
    handleControlPointMouseEvent?.(index, !!isExtrudedControlPoint, "click");
  }, [index, isExtrudedControlPoint, handleControlPointMouseEvent]);

  return <Entity {...options} onMouseDown={handleMouseDown} onClick={handleMouseClick} />;
};

function getDrawPointImage(): HTMLCanvasElement {
  if (drawPointImage != null) {
    return drawPointImage;
  }
  drawPointImage = document.createElement("canvas");
  drawPointImage.width = 16;
  drawPointImage.height = 16;
  const ctx = drawPointImage.getContext("2d");
  invariant(ctx != null);
  ctx.fillStyle = "white";
  ctx.fillRect(3, 3, 10, 10);
  ctx.strokeStyle = "black";
  ctx.lineWidth = 2;
  ctx.strokeRect(3, 3, 10, 10);
  return drawPointImage;
}

function getEditPointImage(): HTMLCanvasElement {
  return getImage(editPointImage, {
    size: 32,
    color: "white",
    strokeColor: DEFAULT_EDIT_COLOR,
  });
}

function getSelectedPointImage(): HTMLCanvasElement {
  return getImage(selectedPointImage, {
    size: 32,
    color: "white",
    strokeColor: SELECTED_EDIT_COLOR,
  });
}

function getAddingPointImage(): HTMLCanvasElement {
  if (addingPointImage != null) {
    return addingPointImage;
  }
  addingPointImage = document.createElement("canvas");
  addingPointImage.width = 32;
  addingPointImage.height = 32;
  const ctx = addingPointImage.getContext("2d");
  invariant(ctx != null);
  ctx.beginPath();
  ctx.arc(16, 16, 12, 0, 2 * Math.PI);
  ctx.fillStyle = "white";
  ctx.fill();
  ctx.strokeStyle = ADDING_POINT_COLOR;
  ctx.lineWidth = 4;
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(16, 8);
  ctx.lineTo(16, 24);
  ctx.stroke();
  ctx.moveTo(8, 16);
  ctx.lineTo(24, 16);
  ctx.stroke();
  return addingPointImage;
}

function getImage(
  canvas: HTMLCanvasElement | undefined,
  options: {
    size: number;
    color: string;
    strokeColor: string;
  },
): HTMLCanvasElement {
  if (canvas != null) {
    return canvas;
  }
  canvas = document.createElement("canvas");
  canvas.width = options.size;
  canvas.height = options.size;
  const ctx = canvas.getContext("2d");
  invariant(ctx != null);
  ctx.beginPath();
  ctx.arc(options.size / 2, options.size / 2, options.size / 2 - 4, 0, 2 * Math.PI);
  ctx.fillStyle = options.color;
  ctx.fill();
  ctx.strokeStyle = options.strokeColor;
  ctx.lineWidth = 4;
  ctx.stroke();
  return canvas;
}
