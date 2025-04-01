import { FC } from "react";

import { composeIdentifier } from "../../prototypes/cesium-helpers";
import {
  ScreenSpaceSelectionEntry,
  useScreenSpaceSelectionResponder,
} from "../../prototypes/screen-space-selection";

import { SPATIAL_ID_OBJECT } from "./types";

export interface SpatialIdObjectProps {
  id: string;
}

export const SpatialIdObject: FC<SpatialIdObjectProps> = ({ id }) => {
  const objectId = composeIdentifier({
    type: "SpatialId",
    subtype: SPATIAL_ID_OBJECT,
    key: id,
  });

  useScreenSpaceSelectionResponder({
    type: SPATIAL_ID_OBJECT,
    convertToSelection: object => {
      return "properties" in object &&
        object.properties &&
        typeof object.properties === "object" &&
        "id" in object.properties &&
        object.properties.id === id
        ? {
            type: SPATIAL_ID_OBJECT,
            value: objectId,
          }
        : undefined;
    },
    shouldRespondToSelection: (
      value,
    ): value is ScreenSpaceSelectionEntry<typeof SPATIAL_ID_OBJECT> => {
      return value.type === SPATIAL_ID_OBJECT && value.value === objectId;
    },
  });

  return null;
};
