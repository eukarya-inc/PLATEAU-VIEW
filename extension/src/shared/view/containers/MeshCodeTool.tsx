import { useAtom } from "jotai";
import { FC, useMemo } from "react";

import { toolAtom } from "../../../prototypes/view/states/tool";
import MeshCodeDrawer from "../../meshCode/MeshCodeDrawer";

export const MeshCodeTool: FC = () => {
  const [toolType] = useAtom(toolAtom);
  const enabled = useMemo(() => toolType?.type === "meshCode", [toolType?.type]);

  return enabled ? <MeshCodeDrawer /> : null;
};
