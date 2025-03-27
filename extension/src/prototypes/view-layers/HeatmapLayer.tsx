import { SetStateAction, atom, useAtomValue, useSetAtom, type PrimitiveAtom } from "jotai";
import { memo, useCallback, useEffect, useMemo, useRef, useState, type FC } from "react";
import invariant from "tiny-invariant";

import {
  HeatmapAppearances,
  HeatmapLayer as ReEarthHeatmapLayer,
} from "../../shared/reearth/layers";
import { LayerVisibilityEvent, ReEarthV1 } from "../../shared/reearth/types";
import { ReEarthV2 } from "../../shared/reearth/types/reearthPluginAPIv2";
import { isReEarthAPIv2 } from "../../shared/reearth/utils/reearth";
import { useStaticApiUrl } from "../../shared/states/environmentVariables";
import { makeComponentAtomWrapper } from "../../shared/view-layers/component";
import { colorMapFlare, ColorMap, createColorMapFromType } from "../color-maps";
import {
  createMeshImageData,
  type MeshImageData,
  type ParseCSVOptions,
  type ParseCSVResult,
  parseCSVAsync,
  createMeshDataAsync,
} from "../heatmap";
import { type LayerProps } from "../layers";
import { MeshBounds, convertCodeToBounds, inferMeshType } from "../regional-mesh";

import {
  createViewLayerModel,
  type ViewLayerModel,
  type ViewLayerModelParams,
} from "./createViewLayerModel";
import { HEATMAP_LAYER } from "./layerTypes";
import { type ConfigurableLayerModel, type LayerColorScheme } from "./types";

export interface HeatmapLayerModelParams extends ViewLayerModelParams {
  title?: string;
  getUrl: (code: string) => string | undefined;
  codes: readonly string[];
  parserOptions: ParseCSVOptions;
  opacity?: number;
  datasetId: string;
  dataId: string;
  shouldInitializeAtom?: boolean;
  shareId?: string;
}

export interface HeatmapLayerModel extends ViewLayerModel {
  getUrl: (code: string) => string | undefined;
  codes: readonly string[];
  parserOptions: ParseCSVOptions;
  opacityAtom: PrimitiveAtom<number>;
  valueRangeAtom: PrimitiveAtom<number[]>;
  contourSpacingAtom: PrimitiveAtom<number>;
  datasetId: string;
  dataId: string;
}

export type SharedHeatmapLayer = {
  type: "heatmap";
  id: string;
  datasetId: string;
  dataId: string;
};

export function createHeatmapLayer(
  params: HeatmapLayerModelParams,
): ConfigurableLayerModel<HeatmapLayerModel> {
  const originalColorMapAtom = atom<ColorMap>(colorMapFlare);
  const wrappedOriginalColorMapAtom = atom(
    get => get(originalColorMapAtom),
    (get, set, colorMapAction: SetStateAction<ColorMap>) => {
      const colorMap =
        typeof colorMapAction === "function"
          ? colorMapAction(get(originalColorMapAtom))
          : colorMapAction;
      if (colorMap instanceof ColorMap) {
        set(originalColorMapAtom, colorMap);
        return;
      }
      const objectColorMap: unknown = colorMap;
      if (typeof objectColorMap === "string") {
        set(originalColorMapAtom, createColorMapFromType(objectColorMap) ?? colorMapFlare);
      }
    },
  );

  const colorMapAtom = makeComponentAtomWrapper(
    wrappedOriginalColorMapAtom,
    {
      ...params,
      componentType: "colorMap",
    },
    true,
    {
      shouldInitialize: params.shouldInitializeAtom,
      beforeSet: a => (a instanceof ColorMap ? a.name : typeof a === "string" ? a : undefined),
    },
  );
  const colorRangeAtom = makeComponentAtomWrapper(
    atom([0, 100]),
    {
      ...params,
      componentType: "colorRange",
    },
    true,
    { shouldInitialize: params.shouldInitializeAtom },
  );
  const valueRangeAtom = makeComponentAtomWrapper(
    atom([0, 100]),
    {
      ...params,
      componentType: "valueRange",
    },
    false,
    { shouldInitialize: params.shouldInitializeAtom },
  );
  const colorSchemeAtom = atom<LayerColorScheme | null>({
    type: "quantitative",
    name: params.title ?? "統計データ",
    colorMapAtom,
    colorRangeAtom,
    valueRangeAtom,
  });
  const opacityAtom = makeComponentAtomWrapper(
    atom(params.opacity ?? 0.8),
    {
      ...params,
      componentType: "opacity",
    },
    false,
    { shouldInitialize: params.shouldInitializeAtom },
  );
  const contourSpacingAtom = makeComponentAtomWrapper(
    atom(10),
    {
      ...params,
      componentType: "contourSpacing",
    },
    false,
    { shouldInitialize: params.shouldInitializeAtom },
  );

  return {
    ...createViewLayerModel({
      ...params,
      title: params.title ?? "統計データ",
    }),
    type: HEATMAP_LAYER,
    getUrl: params.getUrl,
    codes: params.codes,
    parserOptions: params.parserOptions,
    opacityAtom,
    valueRangeAtom,
    contourSpacingAtom,
    colorSchemeAtom,
    datasetId: params.datasetId,
    dataId: params.dataId,
  };
}

const Subdivision: FC<
  Omit<HeatmapLayerModel, "getUrl" | "codes"> & {
    baseUrl: string;
    url: string;
    bound: MeshBounds;
    onLoad?: (data: ParseCSVResult, url: string) => void;
  }
> = memo(
  ({
    baseUrl,
    hiddenAtom,
    colorSchemeAtom,
    url,
    bound,
    parserOptions,
    opacityAtom,
    contourSpacingAtom,
    onLoad,
  }) => {
    const [data, setData] = useState<ParseCSVResult>();
    const [layerIdCurrent, setLayerIdCurrent] = useState<string | null>(null);
    const onLoadRef = useRef(onLoad);
    onLoadRef.current = onLoad;

    const handleOnHeatMapLoad = useCallback((layerId: string) => {
      setLayerIdCurrent(layerId);
    }, []);

    useEffect(() => {
      let isCancelled = false;

      const handleLayerVisibility = (e: LayerVisibilityEvent): void => {
        if (layerIdCurrent === e.layerId) {
          const fetchData = async () => {
            try {
              const response = await fetch(`${baseUrl}/${url}`).then(r => r.text());
              if (isCancelled || !response) return;
              const parsedData = await parseCSVAsync(response, parserOptions);
              if (isCancelled) return;

              setData(parsedData);
              onLoadRef.current?.(parsedData, url);
            } catch (error) {
              console.error(error);
            }
          };
          fetchData();
        }
      };

      if (layerIdCurrent) {
        if (isReEarthAPIv2(window.reearth)) {
          window.reearth?.layers?.on("visible", handleLayerVisibility);
          return () => {
            isCancelled = true;
            (window.reearth as ReEarthV2)?.layers?.off("visible", handleLayerVisibility);
          };
        } else {
          const eventKey = "layerVisibility";
          window.reearth?.on?.(eventKey, handleLayerVisibility);
          return () => {
            isCancelled = true;
            (window.reearth as ReEarthV1)?.off?.(eventKey, handleLayerVisibility);
          };
        }
      }
    }, [layerIdCurrent, url, baseUrl, parserOptions]);

    const [meshImageData, setMeshImageData] = useState<MeshImageData>();
    useEffect(() => {
      if (data == null) {
        setMeshImageData(undefined);
        return;
      }
      let canceled = false;
      (async () => {
        const meshData = await createMeshDataAsync(data);
        if (canceled) {
          return;
        }
        const meshImageData = createMeshImageData(meshData);
        setMeshImageData(meshImageData);
      })().catch(error => {
        console.error(error);
      });
      return () => {
        canceled = true;
      };
    }, [data]);

    const colorScheme = useAtomValue(colorSchemeAtom);
    invariant(colorScheme?.type === "quantitative");
    const colorMap = useAtomValue(colorScheme.colorMapAtom);
    const colorRange = useAtomValue(colorScheme.colorRangeAtom);
    const contourSpacing = useAtomValue(contourSpacingAtom);

    const hidden = useAtomValue(hiddenAtom);
    const opacity = useAtomValue(opacityAtom);

    const appearances: HeatmapAppearances = useMemo(
      () => ({
        heatMap: {
          valueMap: meshImageData?.image.toDataURL(),
          bounds: meshImageData?.bounds || bound,
          colorMap: colorMap.lut,
          opacity,
          minValue: colorRange[0],
          maxValue: colorRange[1],
          contourSpacing,
        },
      }),
      [
        meshImageData?.image,
        meshImageData?.bounds,
        bound,
        colorMap.lut,
        opacity,
        colorRange,
        contourSpacing,
      ],
    );

    if (hidden) {
      return null;
    }
    return <ReEarthHeatmapLayer appearances={appearances} onLoad={handleOnHeatMapLoad} />;
  },
);

function extendRange(a: number[], b: number[]): [number, number] {
  invariant(a.length === 2);
  invariant(b.length === 2);
  return [Math.min(a[0], b[0]), Math.max(a[1], b[1])];
}

export const HeatmapLayer: FC<LayerProps<typeof HEATMAP_LAYER>> = ({ getUrl, codes, ...props }) => {
  const colorScheme = useAtomValue(props.colorSchemeAtom);
  invariant(colorScheme?.type === "quantitative");
  const setColorRange = useSetAtom(colorScheme.colorRangeAtom);
  const setValueRange = useSetAtom(colorScheme.valueRangeAtom);
  const setContourSpacing = useSetAtom(props.contourSpacingAtom);
  const [baseUrl] = useStaticApiUrl();

  const handleLoad = useCallback(
    (data: ParseCSVResult) => {
      setValueRange(prevValue => extendRange(prevValue, [0, data.maxValue]));
      setContourSpacing(prevValue => Math.max(prevValue, data.outlierThreshold / 20));
      setColorRange(prevValue => extendRange(prevValue, [0, data.outlierThreshold]));
    },
    [setValueRange, setContourSpacing, setColorRange],
  );

  const propsArray = useMemo(
    () =>
      codes.map(code => {
        const url = getUrl(code);
        const meshType = inferMeshType(code);
        if (url == null || meshType == null) {
          return undefined;
        }
        const bounds = convertCodeToBounds(code, meshType);
        return { url, bounds };
      }),
    [getUrl, codes],
  );
  return (
    <>
      {propsArray.map(
        additionalProps =>
          additionalProps != null && (
            <Subdivision
              key={additionalProps.url}
              baseUrl={baseUrl}
              {...props}
              bound={additionalProps.bounds}
              {...additionalProps}
              onLoad={handleLoad}
            />
          ),
      )}
    </>
  );
};
