import { OperationValue } from "../../editor/containers/ui-components/property/PropertyOperationSelectField";
import { RGBA } from "../types";

export const string = (v: string) => {
  return `"${v}"`;
};

export const color = (v: string, alpha: number) => {
  return v === "transparent" ? `"${v}"` : `color("${v}", ${alpha})`;
};

export const number = (v: number) => {
  return `${v}`;
};

export const numberOrString = (v: number | string) => {
  return typeof v === "number" ? number(v) : string(v);
};

export const variable = (v: string) => {
  return `\${${v}}`;
};

export const isNumber = (prop: string) =>
  `!(${variable(prop)} === "" || ${variable(prop)} === null || isNaN(Number(${variable(prop)})))`;

export const defaultConditionalNumber = (prop: string, defaultValue?: number) =>
  `(${isNumber(prop)} ? Number(${variable(prop)}) : ${defaultValue ?? "null"})`;

export const condition = (cond: string, v: string, el: string) => {
  return `((${cond}) ? ${v} : ${el})`;
};

export const rgba = (rgba: RGBA) => {
  return `rgba(${rgba.r}, ${rgba.g}, ${rgba.b}, ${rgba.a})`;
};

export const conditionWithOperation = (variable: string, value: string, op: OperationValue) => {
  if (op === "startsWith") {
    return `${op}(${variable}, ${value})`;
  }

  return `${variable} ${op} ${value}`;
};
