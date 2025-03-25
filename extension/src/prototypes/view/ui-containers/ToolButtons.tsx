import { useAtom, useAtomValue, useSetAtom } from "jotai";
import { useCallback, type FC } from "react";

import { isMeshCodeType } from "../../../shared/meshCode/types";
import { isSketchGeometryType } from "../../sketch";
import {
  AppToggleButton,
  AppToggleButtonGroup,
  AppToggleButtonSelect,
  HandIcon,
  PedestrianIcon,
  PointerArrowIcon,
  SketchRectangleIcon,
  SketchCircleIcon,
  SketchPolygonIcon,
  MeshLevel2Icon,
  MeshLevel3Icon,
} from "../../ui-components";
import { AppToggleButtonSlider } from "../../ui-components/AppToggleButtonSlider";
import { SpatialIdIcon } from "../../ui-components/icons/SpatialIdIcon";
import {
  meshCodeTypeAtom,
  sketchTypeAtom,
  spatialIdZoomAtom,
  toolAtom,
  toolMachineAtom,
  type ToolType,
} from "../states/tool";
import { type EventObject } from "../states/toolMachine";

const eventTypes: Record<ToolType, EventObject["type"]> = {
  hand: "HAND",
  select: "SELECT",
  sketch: "SKETCH",
  pedestrian: "PEDESTRIAN",
  spatialId: "SPATIAL_ID",
  meshCode: "MESH_CODE",
};

const sketchItems = [
  { value: "rectangle", title: "立方体", icon: <SketchRectangleIcon /> },
  { value: "circle", title: "円柱", icon: <SketchCircleIcon /> },
  { value: "polygon", title: "自由形状", icon: <SketchPolygonIcon /> },
];

const meshCodeItems = [
  { value: "2x", title: "２次メッシュ", icon: <MeshLevel2Icon /> },
  { value: "3x", title: "３次メッシュ", icon: <MeshLevel3Icon /> },
];

export const ToolButtons: FC = () => {
  const send = useSetAtom(toolMachineAtom);
  const tool = useAtomValue(toolAtom);

  const handleChange = useCallback(
    (_event: unknown, value: ToolType | null) => {
      if (value != null) {
        send({ type: eventTypes[value] });
      }
    },
    [send],
  );

  const [sketchType, setSketchType] = useAtom(sketchTypeAtom);
  const handleSketchTypeChange = useCallback(
    (_: unknown, value: string) => {
      if (isSketchGeometryType(value)) {
        send({ type: "SKETCH" });
        setSketchType(value);
      }
    },
    [send, setSketchType],
  );

  const [spatialIdZoom, setSpatialIdZoom] = useAtom(spatialIdZoomAtom);

  const [meshCodeType, setMeshCodeType] = useAtom(meshCodeTypeAtom);
  const handleMeshCodeTypeChange = useCallback(
    (_: unknown, value: string) => {
      if (isMeshCodeType(value)) {
        send({ type: "MESH_CODE" });
        setMeshCodeType(value);
      }
    },
    [send, setMeshCodeType],
  );

  return (
    <AppToggleButtonGroup value={tool?.type} onChange={handleChange}>
      <AppToggleButton value="hand" title="移動" shortcutKey="H">
        <HandIcon fontSize="medium" />
      </AppToggleButton>
      <AppToggleButton value="select" title="選択" shortcutKey="V">
        <PointerArrowIcon fontSize="medium" />
      </AppToggleButton>
      <AppToggleButton value="pedestrian" title="歩行者視点" shortcutKey="P">
        <PedestrianIcon fontSize="medium" />
      </AppToggleButton>
      <AppToggleButtonSelect
        value="sketch"
        title="作図"
        shortcutKey="G"
        items={sketchItems}
        selectedValue={sketchType}
        onValueChange={handleSketchTypeChange}
      />
      <AppToggleButtonSlider
        value="spatialId"
        title="空間 ID"
        shortcutKey="I"
        item={{
          title: "Zoom Level",
          value: spatialIdZoom,
          min: 16,
          max: 20,
          onValueChange: setSpatialIdZoom,
        }}>
        <SpatialIdIcon fontSize="medium" />
      </AppToggleButtonSlider>
      <AppToggleButtonSelect
        value="meshCode"
        title="Mesh"
        shortcutKey="M"
        items={meshCodeItems}
        selectedValue={meshCodeType}
        onValueChange={handleMeshCodeTypeChange}
      />
    </AppToggleButtonGroup>
  );
};
