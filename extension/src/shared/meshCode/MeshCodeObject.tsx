import { FC } from "react";

import { composeIdentifier } from "../../prototypes/cesium-helpers";
import {
  ScreenSpaceSelectionEntry,
  useScreenSpaceSelectionResponder,
} from "../../prototypes/screen-space-selection";

import { MESH_CODE_OBJECT } from "./types";

export interface MeshCodeObjectProps {
  id: string;
}

export const MeshCodeObject: FC<MeshCodeObjectProps> = ({ id }) => {
  const objectId = composeIdentifier({
    type: "MeshCode",
    subtype: MESH_CODE_OBJECT,
    key: id,
  });

  useScreenSpaceSelectionResponder({
    type: MESH_CODE_OBJECT,
    convertToSelection: object => {
      return "properties" in object &&
        object.properties &&
        typeof object.properties === "object" &&
        "id" in object.properties &&
        object.properties.id === id
        ? {
            type: MESH_CODE_OBJECT,
            value: objectId,
          }
        : undefined;
    },
    shouldRespondToSelection: (
      value,
    ): value is ScreenSpaceSelectionEntry<typeof MESH_CODE_OBJECT> => {
      return value.type === MESH_CODE_OBJECT && value.value === objectId;
    },
  });

  return null;
};
