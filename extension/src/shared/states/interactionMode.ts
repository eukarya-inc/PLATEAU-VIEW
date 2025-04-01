import { atom } from "jotai";

import { InteractionModeType } from "../reearth/types";
import { ReEarthV2 } from "../reearth/types/reearthPluginAPIv2";
import { isReEarthAPIv2 } from "../reearth/utils/reearth";
import { sharedAtom } from "../sharedAtoms";

export const interactionModeAtom = sharedAtom<InteractionModeType>("interactionMode", "move");
export const reearthInteractionModeAtom = atom(null, (_get, set, update: InteractionModeType) => {
  if ((window.reearth as ReEarthV2)?.viewer?.interactionMode?.mode === update) return;
  isReEarthAPIv2(window.reearth)
    ? window.reearth?.viewer?.interactionMode?.override?.(update)
    : window.reearth?.interactionMode?.override?.(update);
  set(interactionModeAtom, async () => update);
});
