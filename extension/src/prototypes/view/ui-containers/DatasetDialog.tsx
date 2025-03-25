import {
  Button,
  Dialog,
  DialogContent,
  DialogContentText,
  Divider,
  listItemSecondaryActionClasses,
  styled,
  type DialogProps,
  listItemTextClasses,
  Link,
} from "@mui/material";
import { atom, useAtomValue, useSetAtom } from "jotai";
import { useCallback, useMemo, type FC } from "react";

import { DatasetFragmentFragment } from "../../../shared/graphql/types/catalog";
import { rootLayersLayersAtom } from "../../../shared/states/rootLayer";
import { settingsAtom } from "../../../shared/states/setting";
import { templatesAtom } from "../../../shared/states/template";
import { createRootLayerForDatasetAtom } from "../../../shared/view-layers";
import { removeLayerAtom, useAddLayer } from "../../layers";
import {
  EntityTitle,
  ExternalLinkIcon,
  PrefixedAddSmallIcon,
  PrefixedCheckSmallIcon,
  UseCaseIcon,
} from "../../ui-components";
import { datasetTypeIcons } from "../constants/datasetTypeIcons";
import { datasetTypeLayers } from "../constants/datasetTypeLayers";
import { PlateauDatasetType } from "../constants/plateau";

const StyledEntityTitle = styled(EntityTitle)(({ theme }) => ({
  minHeight: theme.spacing(7),
  [`& .${listItemSecondaryActionClasses.root}`]: {
    right: 4,
  },
  [`& .${listItemTextClasses.primary}`]: {
    fontSize: theme.typography.h6.fontSize,
    fontWeight: "bold",
  },
  [`& .${listItemTextClasses.secondary}`]: {
    fontSize: theme.typography.h6.fontSize,
  },
  flexShrink: 0,
}));

const StyledDialogContentText = styled(DialogContentText)(({ theme }) => ({
  color: theme.palette.text.primary,
  whiteSpace: "pre-line",
}));

const StyledButton = styled(Button, {
  shouldForwardProp: prop => prop !== "checked",
})<{
  checked?: boolean;
}>(({ theme, checked = false }) => ({
  fontSize: theme.typography.body2.fontSize,
  ...(checked && {
    color: theme.palette.primary.main,
  }),
}));

const StyledDialog = styled(Dialog)(({ theme }) => ({
  "& .MuiDialog-paper": {
    [theme.breakpoints.down("mobile")]: {
      maxHeight: "50%",
    },
  },
}));

export interface DatasetDialogProps extends Omit<DialogProps, "children"> {
  municipalityCode: string;
  dataset: DatasetFragmentFragment;
  isFolder?: boolean;
}

export const DatasetDialog: FC<DatasetDialogProps> = ({
  dataset,
  municipalityCode,
  isFolder,
  ...props
}) => {
  // TODO: Separate into hook
  const layer = useAtomValue(
    useMemo(
      () => atom(get => get(rootLayersLayersAtom).find(layer => layer.id === dataset.id)),
      [dataset.id],
    ),
  );

  const settings = useAtomValue(settingsAtom);
  const templates = useAtomValue(templatesAtom);

  const layerType =
    datasetTypeLayers[dataset.type.code as PlateauDatasetType] ?? datasetTypeLayers.usecase;
  const addLayer = useAddLayer();
  const removeLayer = useSetAtom(removeLayerAtom);
  const handleClick = useCallback(() => {
    if (layerType == null || !dataset) {
      return;
    }
    if (layer == null) {
      const filteredSettings = settings.filter(s => s.datasetId === dataset.id);
      addLayer(
        createRootLayerForDatasetAtom({
          dataset,
          settings: filteredSettings,
          templates,
          areaCode: municipalityCode,
        }),
      );
    } else {
      removeLayer(layer.id);
    }
  }, [dataset, layer, layerType, addLayer, removeLayer, municipalityCode, settings, templates]);

  return (
    <StyledDialog maxWidth="mobile" {...props}>
      <StyledEntityTitle
        iconComponent={datasetTypeIcons[dataset.type.code as PlateauDatasetType] ?? UseCaseIcon}
        title={{
          primary: [
            dataset.name,
            // Currently we don't want subname here, leaving it for future use
            // dataset.__typename === "PlateauDataset" ? dataset.subname ?? "" : "",
          ].join(" "),
          secondary: dataset?.prefecture?.name,
        }}
        secondaryAction={
          !isFolder && (
            <StyledButton
              variant="contained"
              startIcon={
                layer == null ? (
                  <PrefixedAddSmallIcon fontSize="small" />
                ) : (
                  <PrefixedCheckSmallIcon fontSize="small" />
                )
              }
              checked={layer != null}
              disabled={layerType == null}
              onClick={handleClick}>
              {layer == null ? "追加" : "追加済み"}
            </StyledButton>
          )
        }
        allowWrap
      />
      <Divider />
      <DialogContent>
        <StyledDialogContentText>
          <Linkify content={dataset.description} />
        </StyledDialogContentText>
      </DialogContent>
    </StyledDialog>
  );
};

const Linkify: FC<{ content: string | null | undefined }> = ({ content }) => {
  // Ref: https://github.com/Project-PLATEAU/PLATEAU-VIEW-2.0/blob/e6f9cf3307c60d6d3a2613e34cbdf3eaa2060731/plugin/web/extensions/sidebar/modals/datacatalog/components/content/DatasetsPage/Details.tsx#L42-L55
  return (
    <>
      {content
        ?.split(
          /(https?:\/\/[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9(@:%_+.~#?&//=]*)/,
        )
        .map((e, i) =>
          (i + 1) % 2 === 0 ? (
            <Link key={i} onClick={() => window.open(e, "_blank")}>
              {e}
              <ExternalLinkIcon sx={{ marginLeft: "4px" }} fontSize="small" />
            </Link>
          ) : (
            <span key={i}>{e}</span>
          ),
        )}
    </>
  );
};
