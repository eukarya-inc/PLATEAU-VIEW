import {
  Divider,
  IconButton,
  LinearProgress,
  linearProgressClasses,
  List,
  styled,
  Tooltip,
} from "@mui/material";
import { atom, useAtomValue, useSetAtom } from "jotai";
import { uniq } from "lodash-es";
import { FC, useCallback, useMemo } from "react";

import useCityGMLFiles from "../../../shared/api/citygml/hooks/useCityGMLFiles";
import useCityGMLPacks from "../../../shared/api/citygml/hooks/useCityGMLPacks";
import { DEFAULT_PLATEAU_SPEC_VERSION } from "../../../shared/constants";
import { useOptionalAtomValue } from "../../../shared/hooks";
import { MESH_CODE_OBJECT } from "../../../shared/meshCode";
import { LoadingAnimationIcon } from "../../../shared/ui-components/LoadingAnimationIcon";
import { PropertyActionItem } from "../../../shared/ui-components/PropertyActionItem";
import { parseIdentifier } from "../../cesium-helpers";
import { layerSelectionAtom } from "../../layers";
import { screenSpaceSelectionAtom } from "../../screen-space-selection";
import { isNotNullish } from "../../type-helpers";
import {
  InspectorHeader,
  LayerIcon,
  ParameterList,
  PropertyParameterItem,
  TrashIcon,
  PackageIcon,
  DownloadIcon,
  PackageWarningIcon,
} from "../../ui-components";
import { highlightedMeshCodeLayersAtom, MESH_CODE_LAYER } from "../../view-layers";
import { SCREEN_SPACE_SELECTION, SelectionGroup } from "../states/selection";

const ProgressWrapper = styled("div")(({ theme }) => ({
  width: 80,
  height: 32,
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  paddingRight: theme.spacing(0.5),
}));

const BorderLinearProgress = styled(LinearProgress)(({ theme }) => ({
  height: 8,
  borderRadius: 4,
  [`&.${linearProgressClasses.colorPrimary}`]: {
    borderRadius: 4,
    backgroundColor: theme.palette.grey[200],
  },
  [`& .${linearProgressClasses.bar}`]: {
    borderRadius: 4,
    backgroundColor: theme.palette.primary.main,
  },
}));

const LoadingWrapper = styled("div")(({ theme }) => ({
  display: "flex",
  justifyContent: "center",
  padding: theme.spacing(2),
}));

const PropertyActionsWrapper = styled("div")(({ theme }) => ({
  padding: theme.spacing(1, 0),
}));

const PacksHeader = styled("div")(({ theme }) => ({
  padding: theme.spacing(1, 2),
  display: "flex",
  flexDirection: "column",
  gap: theme.spacing(1),
}));

const PacksTitle = styled("div")(({ theme }) => ({
  fontSize: theme.typography.body2.fontSize,
  color: theme.palette.text.primary,
}));

const PacksDescription = styled("div")(({ theme }) => ({
  fontSize: theme.typography.caption.fontSize,
  color: theme.palette.text.secondary,
}));

const TOOL_TIP_MAX_WIDTH = 167;

export interface MeshCodeObjectContentProps {
  values: (SelectionGroup & {
    type: typeof SCREEN_SPACE_SELECTION;
    subtype: typeof MESH_CODE_OBJECT;
  })["values"];
}

export const MeshCodeObjectContent: FC<MeshCodeObjectContentProps> = ({ values }) => {
  const setSelection = useSetAtom(screenSpaceSelectionAtom);
  const handleClose = useCallback(() => {
    setSelection([]);
  }, [setSelection]);

  const meshCodeLayers = useAtomValue(highlightedMeshCodeLayersAtom);
  const setLayerSelection = useSetAtom(layerSelectionAtom);
  const handleSelectLayers = useCallback(() => {
    setLayerSelection(meshCodeLayers.map(layer => ({ id: layer.id, type: MESH_CODE_LAYER })));
  }, [meshCodeLayers, setLayerSelection]);

  const removeFeatures = useSetAtom(
    useMemo(
      () =>
        atom(null, (get, set, featureIds: readonly string[]) => {
          meshCodeLayers.forEach(meshCodeLayer => {
            const featureAtoms = get(meshCodeLayer.featureAtomsAtom);
            featureIds.forEach(featureId => {
              const featureAtom = featureAtoms.find(
                featureAtom => get(featureAtom).id === featureId,
              );
              if (featureAtom != null) {
                set(meshCodeLayer.featureAtomsAtom, {
                  type: "remove",
                  atom: featureAtom,
                });
              }
            });
          });
        }),
      [meshCodeLayers],
    ),
  );
  const handleRemove = useCallback(() => {
    const featureIds = uniq(values.map(value => parseIdentifier(value).key).filter(isNotNullish));
    removeFeatures(featureIds);
    setSelection([]);
  }, [values, removeFeatures, setSelection]);

  const features = useOptionalAtomValue(meshCodeLayers[0]?.featuresAtom);

  const meshCodeProperties = useMemo(() => {
    const feature = features?.find(feature => parseIdentifier(values[0]).key === feature.id);
    if (!feature) return [];
    return [
      {
        id: "meshCode",
        name: "メッシュコード",
        values: [feature.meshCode],
      },
    ];
  }, [features, values]);

  const meshCodes = useMemo(() => {
    const feature = features?.find(feature => parseIdentifier(values[0]).key === feature.id);
    if (!feature) return [];
    return [feature.meshCode];
  }, [features, values]);

  const { loading, data } = useCityGMLFiles({
    meshIds: meshCodes,
  });

  const { packs, handlePacking, handleDownloadPack } = useCityGMLPacks({ data });

  return (
    <List disablePadding>
      <InspectorHeader
        title={`${values.length}個のメッシュコレクション`}
        iconComponent={LayerIcon}
        actions={
          <>
            <Tooltip title="レイヤーを選択">
              <IconButton aria-label="レイヤーを選択" onClick={handleSelectLayers}>
                <LayerIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="削除">
              <IconButton aria-label="削除" onClick={handleRemove}>
                <TrashIcon />
              </IconButton>
            </Tooltip>
          </>
        }
        onClose={handleClose}
      />
      <Divider />
      <ParameterList>
        <PropertyParameterItem
          properties={meshCodeProperties}
          featureType="tags"
          version={DEFAULT_PLATEAU_SPEC_VERSION}
        />
        {loading ? (
          <LoadingWrapper>
            <LoadingAnimationIcon size={16} />
          </LoadingWrapper>
        ) : (
          <>
            {packs.length > 0 && (
              <>
                <Divider />
                <PropertyActionsWrapper>
                  <PacksHeader>
                    <PacksTitle>CityGMLダウンロード </PacksTitle>
                    <PacksDescription>指定した範囲を含むファイルのダウンロード</PacksDescription>
                  </PacksHeader>
                  {packs.map(item => (
                    <PropertyActionItem key={item.id} name={item.name}>
                      {item.status === "idle" && (
                        <Tooltip
                          title="CityGMLをZIPファイルに圧縮する"
                          PopperProps={{ sx: { maxWidth: TOOL_TIP_MAX_WIDTH } }}>
                          <IconButton
                            aria-label="Pack"
                            onClick={() => handlePacking(item.id)}
                            size="small">
                            <PackageIcon />
                          </IconButton>
                        </Tooltip>
                      )}
                      {item.status === "retry" && (
                        <Tooltip
                          title="Something wrong happened, click icon to pack again"
                          PopperProps={{ sx: { maxWidth: TOOL_TIP_MAX_WIDTH } }}>
                          <IconButton
                            aria-label="Retry"
                            onClick={() => handlePacking(item.id)}
                            size="small">
                            <PackageWarningIcon />
                          </IconButton>
                        </Tooltip>
                      )}
                      {(item.status === "requesting" || item.status === "polling") && (
                        <ProgressWrapper>
                          <BorderLinearProgress
                            variant="determinate"
                            value={item.progress * 100}
                            sx={{ width: "100%" }}
                          />
                        </ProgressWrapper>
                      )}
                      {item.status === "packed" && (
                        <Tooltip
                          title="パッケージングが完了しました。ダウンロードしてください。"
                          PopperProps={{ sx: { maxWidth: TOOL_TIP_MAX_WIDTH } }}>
                          <IconButton
                            aria-label="Download"
                            onClick={() => handleDownloadPack(item.id)}
                            size="small">
                            <DownloadIcon />
                          </IconButton>
                        </Tooltip>
                      )}
                    </PropertyActionItem>
                  ))}
                </PropertyActionsWrapper>
              </>
            )}
          </>
        )}
      </ParameterList>
    </List>
  );
};
