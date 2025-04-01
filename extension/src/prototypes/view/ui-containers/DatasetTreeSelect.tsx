import { Typography, type SelectChangeEvent } from "@mui/material";
import { atom, useAtom, useAtomValue, useSetAtom, type SetStateAction } from "jotai";
import { differenceBy, groupBy } from "lodash";
import { memo, useCallback, useMemo, type FC } from "react";
import invariant from "tiny-invariant";

import { rootLayersAtom } from "../../../shared/states/rootLayer";
import { settingsAtom } from "../../../shared/states/setting";
import { templatesAtom } from "../../../shared/states/template";
import {
  RootLayerConfigForDataset,
  createRootLayerForDatasetAtom,
} from "../../../shared/view-layers";
import { removeLayerAtom, useAddLayer } from "../../layers";
import { isNotNullish } from "../../type-helpers";
import { ContextSelect, SelectGroupItem, SelectItem } from "../../ui-components";
import { showDataFormatsAtom } from "../states/app";
import { DatasetItem } from "../utils/datasetGroups";

interface Params {
  datasetId: string;
  datumId: string;
}

type DefaultDataIdOfDatasetMap = { [K in string]: string | undefined };

function createParamsArray(
  layers: readonly RootLayerConfigForDataset[],
  defaultDataIdOfDatasetMap: DefaultDataIdOfDatasetMap,
): Params[] {
  return layers
    .map(({ id }) => {
      const datumId = defaultDataIdOfDatasetMap[id];
      return datumId != null ? { datasetId: id, datumId } : undefined;
    })
    .filter(isNotNullish);
}

function serializeParams({ datasetId, datumId }: Params): string {
  return JSON.stringify([datasetId, datumId]);
}

function parseParams(value: string): Params {
  const [datasetId, datumId] = JSON.parse(value);
  return { datasetId, datumId };
}

export interface DatasetTreeSelectProps {
  label: string;
  datasets: DatasetItem[];
  municipalityCode: string;
  disabled?: boolean;
  allowContinuousAdd?: boolean;
}

// This component just shows only datasets, not dataset's items.
export const DatasetTreeSelect: FC<DatasetTreeSelectProps> = memo(
  ({ datasets, municipalityCode, disabled, label, allowContinuousAdd }) => {
    invariant(datasets.length > 0);
    const rootLayers = useAtomValue(rootLayersAtom);
    const settings = useAtomValue(settingsAtom);
    const templates = useAtomValue(templatesAtom);

    const datasetIds = useMemo(() => datasets.map(d => d.id), [datasets]);
    // This component just shows only datasets, so datumId should be first one always even if actual datumId is different.
    const defaultDataIdOfDatasetMap = useMemo(
      () =>
        datasets.reduce(
          (r, d) => ({ ...r, [d.id]: d.items[0]?.id }),
          {} as DefaultDataIdOfDatasetMap,
        ),
      [datasets],
    );

    const filteredRootLayers = useMemo(
      () =>
        rootLayers.filter(
          (l): l is RootLayerConfigForDataset => l.type === "dataset" && datasetIds.includes(l.id),
        ),
      [rootLayers, datasetIds],
    );

    const addLayer = useAddLayer();
    const removeLayer = useSetAtom(removeLayerAtom);
    const paramsAtom = useMemo(() => {
      return atom(
        () => createParamsArray(filteredRootLayers, defaultDataIdOfDatasetMap),
        (_get, set, dataIds: SetStateAction<Params[]>) => {
          const prevParams = createParamsArray(filteredRootLayers, defaultDataIdOfDatasetMap);
          const nextParams = typeof dataIds === "function" ? dataIds(prevParams) : dataIds;

          const paramsToRemove = differenceBy(prevParams, nextParams, ({ datasetId }) => datasetId);
          const paramsToAdd = differenceBy(nextParams, prevParams, ({ datasetId }) => datasetId);
          const paramsToUpdate = nextParams.filter(({ datasetId }) =>
            prevParams.some(params => params.datasetId === datasetId),
          );
          paramsToRemove.forEach(({ datasetId }) => {
            const layer = filteredRootLayers.find(({ id }) => id === datasetId);
            invariant(layer != null);
            removeLayer(layer.id);
          });
          paramsToAdd.forEach(({ datasetId, datumId }) => {
            const dataset = datasets.find(d => d.id === datasetId);
            const filteredSettings = settings.filter(s => s.datasetId === datasetId);
            if (!dataset) {
              return;
            }
            addLayer(
              createRootLayerForDatasetAtom({
                dataset,
                areaCode: municipalityCode,
                settings: filteredSettings,
                templates,
                currentDataId: datumId,
              }),
            );
          });
          paramsToUpdate.forEach(({ datasetId, datumId }) => {
            const layer = filteredRootLayers.find(layer => layer.id === datasetId);
            invariant(layer != null);
            set(layer.currentDataIdAtom, datumId);
          });
        },
      );
    }, [
      municipalityCode,
      filteredRootLayers,
      datasets,
      addLayer,
      removeLayer,
      settings,
      templates,
      defaultDataIdOfDatasetMap,
    ]);

    const [params, setParams] = useAtom(paramsAtom);

    const handleChange = useCallback(
      (event: SelectChangeEvent<string[]>) => {
        invariant(Array.isArray(event.target.value));
        setParams(event.target.value.map(value => parseParams(value)));
      },
      [setParams],
    );

    const value = useMemo(
      () => (params != null ? params.map(params => serializeParams(params)) : []),
      [params],
    );

    const showDataFormats = useAtomValue(showDataFormatsAtom);

    const selectTreeItems: SelectTreeItem[] = useMemo(() => {
      const list: SelectTreeItem[] = [];
      processDatasets(datasets, 0, list);
      return list;
    }, [datasets]);

    return (
      <ContextSelect
        label={label}
        value={value}
        onChange={handleChange}
        disabled={disabled}
        autoClose={!allowContinuousAdd}>
        {selectTreeItems.map((item, index) => {
          if (item.isFolder) {
            return (
              <SelectGroupItem key={index} size="small" sx={{ paddingLeft: 3.5 + item.level * 3 }}>
                {item.label}
              </SelectGroupItem>
            );
          }
          if (!item.dataset?.items.length) return;
          return (
            <SelectItem
              key={index}
              value={serializeParams({
                datasetId: item.dataset?.id ?? "",
                datumId: item.dataset?.items[0].id ?? "",
              })}
              indent={item.level * 1.5}>
              <Typography variant="body2">
                {(item.dataset?.folderPath ?? item.dataset?.name)?.split("/").pop()}
                {showDataFormats ? ` (${item.dataset?.items[0].format})` : null}
              </Typography>
            </SelectItem>
          );
        })}
      </ContextSelect>
    );
  },
);

type SelectTreeItem = {
  isFolder: boolean;
  level: number;
  label?: string;
  dataset?: DatasetItem;
};

function processDatasets(datasets: DatasetItem[], level: number, result: SelectTreeItem[]) {
  Object.entries(groupBy(datasets, d => (d.folderPath ?? d.name).split("/")[level])).forEach(
    ([key, value]) => {
      if (key !== "undefined") {
        if (
          value.length === 1 &&
          (value[0].folderPath ?? value[0].name).split("/").length <= level + 1
        ) {
          if (value[0].items.length === 0) return;
          result.push({
            isFolder: false,
            level,
            dataset: value[0],
          });
          return;
        }
        result.push({
          isFolder: true,
          level,
          label: key,
        });
        processDatasets(value, level + 1, result);
      } else {
        value.forEach(v => {
          if (v.items.length === 0) return;
          result.push({
            isFolder: false,
            level,
            dataset: v,
          });
        });
      }
    },
  );
}
