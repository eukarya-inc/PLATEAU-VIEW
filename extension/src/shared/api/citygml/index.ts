import { CityGMLClient } from "./client";

export let cityGMLClient: CityGMLClient | undefined;

export const createCityGMLClient = (url: string) => {
  cityGMLClient = new CityGMLClient(url);
};
