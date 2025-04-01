import jismesh from "jismesh-js";
import { useAtom } from "jotai";
import { FC, useCallback, useState } from "react";

import { meshCodeTypeAtom } from "../../prototypes/view/states/tool";
import { useReEarthEvent } from "../reearth/hooks";
import { MeshCodeIndicator } from "../reearth/layers";
import { MouseEvent } from "../reearth/types";

import useMeshCode from "./useMeshCode";
import { getCoordinatesFromMeshCode, getMeshCodeLevelByType } from "./utils";

const MeshCodeDrawer: FC = () => {
  const [meshCodeType] = useAtom(meshCodeTypeAtom);

  const [coordinates, setCoordinates] = useState<[number, number][][] | undefined>();

  const { handleCreate } = useMeshCode({ meshCodeType });

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!e.lat || !e.lng) return;
      const meshCode = jismesh.toMeshCode(e.lat, e.lng, getMeshCodeLevelByType(meshCodeType));
      if (!meshCode) return;
      setCoordinates(getCoordinatesFromMeshCode(meshCode));
    },
    [meshCodeType],
  );

  const handleMouseClick = useCallback(
    (e: MouseEvent) => {
      if (!e.lat || !e.lng) return;
      handleCreate({ location: { lat: e.lat, lng: e.lng } });
    },
    [handleCreate],
  );

  useReEarthEvent("mousemove", handleMouseMove);
  useReEarthEvent("click", handleMouseClick);

  return coordinates ? <MeshCodeIndicator coordinates={coordinates} /> : null;
};

export default MeshCodeDrawer;
