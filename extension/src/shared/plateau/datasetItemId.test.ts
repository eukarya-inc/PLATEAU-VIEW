import { expect, test } from "vitest";

import {
  findDatasetItemById,
  findSettingForItem,
  hasItemsToShow,
  toChoiceItemId,
  toCrsAgnosticItemId,
} from "./datasetItemId";

test("toCrsAgnosticItemId", () => {
  expect(toCrsAgnosticItemId("d_13101_bldg_lod2")).toBe("d_13101_bldg_lod2");
  expect(toCrsAgnosticItemId("d_13101_bldg_lod2_jgd2024")).toBe("d_13101_bldg_lod2");
  expect(toCrsAgnosticItemId("d_13101_bldg_lod2_no_texture_jgd2024_full")).toBe(
    "d_13101_bldg_lod2_no_texture_full",
  );
  expect(toCrsAgnosticItemId("d_13101_bldg_lod2_full")).toBe("d_13101_bldg_lod2_full");
  // only the CRS segment of the item, not a match elsewhere in the ID
  expect(toCrsAgnosticItemId("d_jgd2024_bldg_lod2")).toBe("d_jgd2024_bldg_lod2");
});

test("findDatasetItemById", () => {
  const jgd2011 = [{ id: "d_bldg_lod1" }, { id: "d_bldg_lod2" }];
  const jgd2024 = [{ id: "d_bldg_lod1_jgd2024" }, { id: "d_bldg_lod2_jgd2024" }];

  expect(findDatasetItemById(jgd2011, "d_bldg_lod2")).toBe(jgd2011[1]);
  expect(findDatasetItemById(jgd2024, "d_bldg_lod2")).toBe(jgd2024[1]);
  expect(findDatasetItemById(jgd2011, "d_bldg_lod2_jgd2024")).toBe(jgd2011[1]);
  expect(findDatasetItemById(jgd2011, "d_bldg_lod3")).toBeUndefined();
  expect(findDatasetItemById(jgd2011, undefined)).toBeUndefined();
  // an exact match wins over a match in another CRS
  const mixed = [{ id: "d_mvt" }, { id: "d_bldg_lod2" }, { id: "d_bldg_lod2_jgd2024" }];
  expect(findDatasetItemById(mixed, "d_bldg_lod2_jgd2024")).toBe(mixed[2]);
});

test("findDatasetItemById across attribute sets", () => {
  // the catalog returns the _full variant only when the choice has no default one in that CRS
  const jgd2024 = [{ id: "d_bldg_lod1_jgd2024" }, { id: "d_bldg_lod2_jgd2024_full" }];
  expect(findDatasetItemById(jgd2024, "d_bldg_lod2")).toBe(jgd2024[1]);
  const jgd2011 = [{ id: "d_bldg_lod2_full" }];
  expect(findDatasetItemById(jgd2011, "d_bldg_lod2_jgd2024")).toBe(jgd2011[0]);
  // a match in the same attribute set wins
  const both = [{ id: "d_bldg_lod2_full" }, { id: "d_bldg_lod2" }];
  expect(findDatasetItemById(both, "d_bldg_lod2_jgd2024")).toBe(both[1]);
  expect(toChoiceItemId("d_bldg_lod2_jgd2024_full")).toBe("d_bldg_lod2");
});

test("findSettingForItem", () => {
  const settings = [
    { datasetId: "d", dataId: "d_bldg_lod2_full" },
    { datasetId: "d", dataId: "d_bldg_lod2" },
    { datasetId: "e", dataId: "d_bldg_lod1" },
  ];
  expect(findSettingForItem(settings, "d", "d_bldg_lod2")).toBe(settings[1]);
  expect(findSettingForItem(settings, "d", "d_bldg_lod2_jgd2024")).toBe(settings[1]);
  expect(findSettingForItem(settings, "d", "d_bldg_lod2_jgd2024_full")).toBe(settings[0]);
  expect(findSettingForItem(settings, "d", "d_bldg_lod1_jgd2024")).toBeUndefined();
  expect(findSettingForItem(settings, undefined, "d_bldg_lod1_jgd2024")).toBe(settings[2]);
  expect(findSettingForItem(settings, "d", undefined)).toBeUndefined();
});

test("hasItemsToShow", () => {
  expect(hasItemsToShow({ __typename: "PlateauDataset", items: [] })).toBe(false);
  expect(hasItemsToShow({ __typename: "PlateauDataset", items: [{}] })).toBe(true);
  expect(hasItemsToShow({ __typename: "GenericDataset", items: [] })).toBe(true);
});
