import { useCallback, useEffect, useState } from "react";

import { cityGMLClient } from "..";
import { isNotNullish } from "../../../../prototypes/type-helpers";
import { CityGMLGetFilesData, CityGMLPackItem } from "../types";

const POLLING_INTERVAL = 2000;
const MAX_POLLING_COUNT = Infinity;

export type UseCityGMLPacksProps = {
  data: CityGMLGetFilesData | null;
};

export default ({ data }: UseCityGMLPacksProps) => {
  const [packs, setPacks] = useState<CityGMLPackItem[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!data) return;

    const packsByType: CityGMLPackItem[] = data.featureTypes
      ? Object.keys(data.featureTypes).map(key => ({
          id: key,
          name: data.featureTypes[key].name,
          status: "idle" as const,
          fileUrls: data.cities
            .map(city => [
              ...(city.files[key]?.map(file => file.url) ?? []),
              ...(city.metadataZipUrls ?? []),
            ])
            .flat(),
          pollingCount: 0,
          progress: 0,
        }))
      : [];

    const totalPack: CityGMLPackItem | null =
      packsByType.length > 0
        ? {
            id: "all",
            name: "全て",
            status: "idle" as const,
            fileUrls: packsByType.flatMap(item => item.fileUrls),
            pollingCount: 0,
            progress: 0,
          }
        : null;

    setPacks([...packsByType, totalPack].filter(isNotNullish));
  }, [data]);

  useEffect(() => {
    if (packs.length === 0) return;

    const intervals: { [key: string]: NodeJS.Timeout } = {};

    packs.forEach(pack => {
      if (pack.status === "polling" && pack.packId) {
        intervals[pack.packId] = setInterval(async () => {
          try {
            if (!pack.packId) return;

            const response = await cityGMLClient?.getPackStatus({ id: pack.packId });
            pack.pollingCount += 1;
            if (!response) return;

            setPacks(prevPacks =>
              prevPacks.map(p =>
                p.id === pack.id
                  ? {
                      ...p,
                      status:
                        response.status === "succeeded"
                          ? "packed"
                          : response.status === "failed" || pack.pollingCount >= MAX_POLLING_COUNT
                          ? "retry"
                          : "polling",
                      progress: response.progress ?? 0,
                    }
                  : p,
              ),
            );

            if (
              response.status === "succeeded" ||
              response.status === "failed" ||
              pack.pollingCount >= MAX_POLLING_COUNT
            ) {
              clearInterval(intervals[pack.packId]);
            }

            if (pack.pollingCount >= MAX_POLLING_COUNT) {
              console.error(`Task ${pack.packId} has been polling for too long.`);
            }
          } catch (error) {
            if (pack.packId) {
              clearInterval(intervals[pack.packId]);
            }
            console.error(`Error checking status for task ${pack.packId}`);
          }
        }, POLLING_INTERVAL);
      }
    });

    return () => {
      Object.values(intervals).forEach(clearInterval);
    };
  }, [packs]);

  const handlePacking = useCallback(
    async (id: string) => {
      const pack = packs.find(pack => pack.id === id);
      if (!pack) return;

      setPacks(prevPacks =>
        prevPacks.map(p =>
          p.id === id
            ? {
                ...p,
                status: "requesting",
              }
            : p,
        ),
      );

      const postPack = async () => {
        try {
          const urls = pack.fileUrls.filter(url => url != null);
          const result = await cityGMLClient?.postPack({ urls });

          setPacks(prevPacks =>
            prevPacks.map(p =>
              p.id === id
                ? {
                    ...p,
                    status: result?.id ? "polling" : "idle",
                    packId: result?.id,
                    pollingCount: 0,
                  }
                : p,
            ),
          );
        } catch (err: any) {
          setError(err.message || "An error occurred while packing files.");
          setPacks(prevPacks =>
            prevPacks.map(p =>
              p.id === id
                ? {
                    ...p,
                    status: "idle",
                    packId: undefined,
                  }
                : p,
            ),
          );
        }
      };

      postPack();
    },
    [packs],
  );

  const handleDownloadPack = useCallback(
    (id: string) => {
      const pack = packs.find(pack => pack.id === id);
      if (!pack?.packId || !cityGMLClient?.url) return;

      const url = `${cityGMLClient.url}/pack/${pack.packId}.zip`;

      const a = document.createElement("a");
      a.href = url;
      a.download = `${pack.name}.zip`;
      a.target = "_blank";
      a.click();
      a.remove();
    },
    [packs],
  );

  return {
    packs,
    error,
    handlePacking,
    handleDownloadPack,
  };
};
