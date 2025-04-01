import { atom } from "jotai";

import {
  screenSpaceSelectionAtom,
  ScreenSpaceSelectionEntry,
} from "../../prototypes/screen-space-selection";

import { SPATIAL_ID_OBJECT } from "./types";

export const spatialIdSelectionAtom = atom(get => {
  return get(screenSpaceSelectionAtom).filter(
    (entry): entry is ScreenSpaceSelectionEntry<typeof SPATIAL_ID_OBJECT> =>
      entry.type === SPATIAL_ID_OBJECT,
  );
});
