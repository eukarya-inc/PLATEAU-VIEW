import { IconButton, Tooltip } from "@mui/material";
import { Stack } from "@mui/system";
import { FC, HTMLAttributes, SyntheticEvent, useCallback, useState } from "react";

import { EntityTitleButton, SearchOption } from "../../prototypes/ui-components";
import { AddMeshIcon, AddressIcon } from "../../prototypes/ui-components/icons";
import {
  getCenterCoordinatesFromMeshCode,
  getMeshCodeFromCoordinates,
} from "../../shared/meshCode/utils";
import { flyToCamera } from "../../shared/reearth/utils";

interface HoverMenuProps {
  hovered?: boolean;
  onAddMeshCodeLayer?: () => void;
}

const HoverMenu: FC<HoverMenuProps> = ({ hovered = false, onAddMeshCodeLayer }) => {
  const handleAddMeshButtonClick = useCallback(
    (event: SyntheticEvent) => {
      event.stopPropagation();
      onAddMeshCodeLayer?.();
    },
    [onAddMeshCodeLayer],
  );

  return hovered ? (
    <Stack direction="row">
      <Tooltip title="メッシュコレクション">
        <span>
          <IconButton
            color="inherit"
            aria-label="メッシュコレクション"
            onClick={handleAddMeshButtonClick}>
            <AddMeshIcon fontSize="small" />
          </IconButton>
        </span>
      </Tooltip>
    </Stack>
  ) : null;
};

export interface SearchOptionAreaProps {
  props: HTMLAttributes<HTMLLIElement>;
  option: SearchOption;
  onCreateMeshCodeLayer: (location: { lat: number; lng: number }) => void;
}

export const SearchOptionAreaButton: FC<SearchOptionAreaProps> = ({
  props,
  option,
  onCreateMeshCodeLayer,
}) => {
  const [hovered, setHovered] = useState(false);
  const handleMouseEnter = () => setHovered(true);
  const handleMouseLeave = () => setHovered(false);

  const handleAddMeshCodeLayer = useCallback(() => {
    if (option.type !== "area" || !option.bbox) return;
    // We can't get mesh code from bbox, so we use the center of bbox as location
    const location = {
      lng: (option.bbox[0] + option.bbox[2]) / 2,
      lat: (option.bbox[1] + option.bbox[3]) / 2,
    };
    onCreateMeshCodeLayer(location);
    // fly to the center of the mesh code feature
    const meshCode = getMeshCodeFromCoordinates(location.lat, location.lng, "3x");
    const meshCodeCenter = getCenterCoordinatesFromMeshCode(meshCode);
    flyToCamera(
      {
        lat: meshCodeCenter[0],
        lng: meshCodeCenter[1],
        height: 5000,
        heading: 0,
        pitch: -Math.PI / 2,
        roll: 0,
      },
      2,
    );
  }, [option, onCreateMeshCodeLayer]);

  return (
    // @ts-expect-error TODO
    <EntityTitleButton
      {...props}
      title={option.displayName ?? option.name}
      iconComponent={AddressIcon}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}>
      <Stack direction="row" spacing={0.5}>
        <HoverMenu hovered={hovered} onAddMeshCodeLayer={handleAddMeshCodeLayer} />
        {props.children}
      </Stack>
    </EntityTitleButton>
  );
};
