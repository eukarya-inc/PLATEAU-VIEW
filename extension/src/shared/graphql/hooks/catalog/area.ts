import { useMemo } from "react";

import { SAMPLE_DATA_CITY_CODE_SUFFIX } from "../../../constants";
import { AREAS, AREA_DATASETS } from "../../base/catalog/queries/area";
import { AreasInput, DatasetsInput } from "../../types/catalog";

import { useQuery } from "./base";

type Options = {
  skip?: boolean;
};

export const useAreas = (input?: AreasInput, options?: Options) => {
  const data = useQuery(AREAS, {
    variables: {
      input: input ?? {},
    },
    skip: options?.skip,
  });
  const next = useMemo(
    () => ({
      ...data,
      data: data.data
        ? {
            ...data.data,
            areas: data.data.areas.filter(a =>
              !(a.code as string).endsWith(SAMPLE_DATA_CITY_CODE_SUFFIX),
            ),
          }
        : undefined,
    }),
    [data],
  );

  return next;
};

export const useAreaDatasets = (code: string, input?: DatasetsInput, options?: Options) => {
  const { data, ...rest } = useQuery(AREA_DATASETS, {
    variables: {
      code,
      input: input ?? {},
    },
    skip: options?.skip,
  });

  const nextDatasets = useMemo(
    () =>
      data?.area?.datasets
        .map(d =>
          (d.cityCode as string).endsWith(SAMPLE_DATA_CITY_CODE_SUFFIX)
            ? { ...d, cityCode: null, city: null }
            : d,
        )
        .sort((a, b) => a.type.order - b.type.order),
    [data],
  );

  return {
    data: data
      ? { ...data, ...(data.area ? { area: { ...data.area, datasets: nextDatasets } } : {}) }
      : undefined,
    ...rest,
  };
};
