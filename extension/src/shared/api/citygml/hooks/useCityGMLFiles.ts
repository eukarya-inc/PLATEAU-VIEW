import { useEffect, useState } from "react";

import { cityGMLClient } from "..";
import { CityGMLGetFilesData, CityGMLGetFilesParams } from "../types";

export type UseCityGMLFilesProps = CityGMLGetFilesParams;

export default ({
  meshIds,
  meshIdsStrict,
  spaceZFXYStrs,
  spaceIds,
  centerCoordinate,
  rangeCoordinates,
  geoName,
  cityIds,
}: UseCityGMLFilesProps) => {
  const [cityNames, setCityNames] = useState<string[] | null>(null);
  const [featureTypes, setFeatureTypes] = useState<{ value: string; name: string }[] | null>(null);
  const [data, setData] = useState<CityGMLGetFilesData | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setCityNames(null);
    setFeatureTypes(null);
    setData(null);
    setLoading(false);
    setError(null);

    const fetchFiles = async () => {
      try {
        setLoading(true);

        const data = await cityGMLClient?.getFiles({
          meshIds,
          meshIdsStrict,
          spaceZFXYStrs,
          spaceIds,
          centerCoordinate,
          rangeCoordinates,
          geoName,
          cityIds,
        });
        if (!data) return;

        setCityNames(data.cities.map(city => city.cityName));

        setFeatureTypes(
          Object.keys(data.featureTypes).map(key => ({
            value: key,
            name: data.featureTypes[key].name,
          })),
        );

        setData(data);
      } catch (err: any) {
        setError(err.message || "An error occurred while fetching files.");
      } finally {
        setLoading(false);
      }
    };

    fetchFiles();
  }, [
    meshIds,
    meshIdsStrict,
    spaceZFXYStrs,
    spaceIds,
    centerCoordinate,
    rangeCoordinates,
    geoName,
    cityIds,
  ]);

  return {
    cityNames,
    featureTypes,
    data,
    loading,
    error,
  };
};
