import { atom } from "jotai";

import {
  screenSpaceSelectionAtom,
  ScreenSpaceSelectionEntry,
} from "../../prototypes/screen-space-selection";

import { MESH_CODE_OBJECT } from "./types";

export const meshCodeSelectionAtom = atom(get => {
  return get(screenSpaceSelectionAtom).filter(
    (entry): entry is ScreenSpaceSelectionEntry<typeof MESH_CODE_OBJECT> =>
      entry.type === MESH_CODE_OBJECT,
  );
});
