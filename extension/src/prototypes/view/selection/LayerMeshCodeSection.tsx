import {
  Divider,
  IconButton,
  LinearProgress,
  linearProgressClasses,
  styled,
  Tooltip,
} from "@mui/material";
import { FC, useMemo } from "react";

import useCityGMLFiles from "../../../shared/api/citygml/hooks/useCityGMLFiles";
import useCityGMLPacks from "../../../shared/api/citygml/hooks/useCityGMLPacks";
import { DEFAULT_PLATEAU_SPEC_VERSION } from "../../../shared/constants";
import { useOptionalAtomValue } from "../../../shared/hooks";
import { LoadingAnimationIcon } from "../../../shared/ui-components/LoadingAnimationIcon";
import { PropertyActionItem } from "../../../shared/ui-components/PropertyActionItem";
import { LayerModel } from "../../layers";
import {
  DownloadIcon,
  InfoIcon,
  PackageIcon,
  PackageWarningIcon,
  ParameterList,
  PropertyParameterItem,
} from "../../ui-components";
import { MESH_CODE_LAYER } from "../../view-layers";

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

const Warning = styled("div")(({ theme }) => ({
  padding: theme.spacing(2),
  color: theme.palette.warning.main,
  fontSize: theme.typography.body2.fontSize,
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  gap: theme.spacing(0.5),
}));

const MAX_MESH_CODES = 9;
const TOOL_TIP_MAX_WIDTH = 167;
export interface LayerMeshCodeSectionProps {
  layers: readonly LayerModel[];
}

export const LayerMeshCodeSection: FC<LayerMeshCodeSectionProps> = ({ layers }) => {
  const meshCodeLayers = useMemo(
    () =>
      layers.filter(
        (layer): layer is LayerModel<typeof MESH_CODE_LAYER> => layer.type === MESH_CODE_LAYER,
      ),
    [layers],
  );

  const features = useOptionalAtomValue(meshCodeLayers[0]?.featuresAtom);

  const meshCodeProperties = useMemo(() => {
    if (!features) return [];
    return [
      {
        id: "meshCode",
        name: "メッシュコード",
        values: features.map(feature => feature.meshCode) ?? [],
      },
    ];
  }, [features]);

  const meshCodes = useMemo(() => {
    return features?.map(feature => feature.meshCode) ?? [];
  }, [features]);

  const { loading, data } = useCityGMLFiles({
    meshIds: meshCodes,
  });

  const { packs, handlePacking, handleDownloadPack } = useCityGMLPacks({ data });

  if (meshCodeLayers.length === 0) {
    return null;
  }
  return (
    <ParameterList>
      <Divider />
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
          {meshCodes.length > MAX_MESH_CODES ? (
            <>
              <Divider />
              <Warning>
                <InfoIcon />
                CityGMLをダウンロードするにはメッシュを９面以下に絞ってください
              </Warning>
            </>
          ) : (
            packs.length > 0 && (
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
            )
          )}
        </>
      )}
    </ParameterList>
  );
};
