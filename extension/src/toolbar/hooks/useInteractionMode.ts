import { useMediaQuery, useTheme } from "@mui/material";
import { useAtomValue, useSetAtom } from "jotai";
import { useEffect } from "react";

import { ToolType, toolAtom } from "../../prototypes/view/states/tool";
import { InteractionModeType } from "../../shared/reearth/types";
import { reearthInteractionModeAtom } from "../../shared/states/interactionMode";

const TOOL_TO_INTERACTIONMODE: Record<ToolType, InteractionModeType> = {
  hand: "move",
  select: "selection",
  sketch: "sketch",
  pedestrian: "move", // TODO: Check later
  spatialId: "spatialId",
  meshCode: "move",
};

export const useInteractionMode = () => {
  const tool = useAtomValue(toolAtom);
  const setInteractionMode = useSetAtom(reearthInteractionModeAtom);
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down("mobile"), { noSsr: true });
  useEffect(() => {
    // There's no tools support on mobile.
    if (isMobile) {
      setInteractionMode("default");
    } else if (tool?.type) {
      setInteractionMode(TOOL_TO_INTERACTIONMODE[tool.type]);
    }
  }, [tool?.type, isMobile, setInteractionMode]);
};
