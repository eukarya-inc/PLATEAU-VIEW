import { groupBy } from "lodash-es";

import { DatasetFragmentFragment } from "../../../shared/graphql/types/catalog";
import { isGenericDatasetType } from "../constants/generic";

export type DatasetItem = DatasetFragmentFragment & { folderPath?: string };

export type DatasetGroupItem = {
  label: string;
  groupId: string;
  datasets: DatasetItem[];
  useTree?: boolean;
  allowContinuousAdd?: boolean;
};

export function getDatasetGroups({
  datasets,
  prefCode,
  cityCode,
  areaCode,
}: {
  datasets: DatasetFragmentFragment[] | undefined;
  prefCode?: number | string;
  cityCode?: number | string;
  areaCode?: number | string;
}): {
  typicalTypeGroups?: DatasetGroupItem[];
  dataGroups?: DatasetGroupItem[];
  cityDatasetGroups?: DatasetGroupItem[];
  genericGroups?: DatasetGroupItem[];
} {
  if (!datasets) return {};

  const typicalTypeDatasets = datasets?.filter(
    d =>
      !(d.groups && d.groups.length > 0) &&
      !isGenericDatasetType(d.type.code) &&
      d.type.code !== "city",
  );
  const typicalTypeGroups = typicalTypeDatasets
    ? Object.entries(groupBy(typicalTypeDatasets, d => d.type.name)).map(([key, value]) => ({
        label: key,
        groupId: generateGroupId("type", key, prefCode, cityCode, areaCode),
        datasets: value.map(v => ({ ...v, folderPath: v.name })),
      }))
    : undefined;

  const dataGroupDatasets = datasets?.filter(d => d.groups && d.groups.length > 0);
  const dataGroups = dataGroupDatasets
    ? Object.entries(groupBy(dataGroupDatasets, d => d.groups?.[0])).map(([key, value]) => ({
        label: key,
        groupId: generateGroupId("group", key, prefCode, cityCode, areaCode),
        datasets: value.map(v => ({
          ...v,
          folderPath:
            v.groups && v.groups.length > 1 ? `${v.groups?.slice(1).join("/")}/${v.name}` : v.name,
        })),
        useTree: true,
      }))
    : undefined;

  const cityDatasets = datasets?.filter(
    d => !(d.groups && d.groups.length > 0) && d.type.code === "city",
  );
  const cityDatasetGroups = cityDatasets
    ? Object.entries(groupBy(cityDatasets, d => d.name.split("/")[0])).map(([key, value]) => ({
        label: key,
        groupId: generateGroupId("city", key, prefCode, cityCode, areaCode),
        datasets: value.map(v => ({ ...v, folderPath: v.name.split("/").slice(1).join("/") })),
        useTree: !(value.length === 1 && !value[0].name.includes("/")),
      }))
    : undefined;

  const genericDatasets = datasets?.filter(
    d =>
      !(d.groups && d.groups.length > 0) &&
      d.type.code !== "city" &&
      isGenericDatasetType(d.type.code),
  );
  const genericGroups = genericDatasets
    ? Object.entries(groupBy(genericDatasets, d => d.type.name)).map(([key, value]) => ({
        label: key,
        groupId: generateGroupId("generic", key, prefCode, cityCode, areaCode),
        datasets: value.map(v => ({ ...v, folderPath: v.name })),
        useTree: true,
        allowContinuousAdd: true,
      }))
    : undefined;

  return { typicalTypeGroups, dataGroups, cityDatasetGroups, genericGroups };
}

function generateGroupId(
  type: string,
  key: string,
  prefCode?: number | string,
  cityCode?: number | string,
  areaCode?: number | string,
) {
  const areas = [areaCode, prefCode, cityCode].filter(a => !!a).join(":");
  return areas ? `${areas}:${type}:${key}` : `${type}:${key}`;
}
