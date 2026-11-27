// The data catalog gives the JGD2024 variant of a PLATEAU dataset item the ID of its JGD2011
// variant plus a `_jgd2024` segment, which stays before a trailing `_full` (see itemID in
// server/datacatalog/datacatalogv3/conv_dataset_plateau_seed.go).
//
// Item IDs are stored outside the catalog: in Editor settings (Setting.dataId, which also
// picks the initial layers), in share links and in the shared store. A project that shows
// one CRS reads IDs that may have been written for the other one. Dropping the segment gives
// a key that every CRS variant of one choice (LOD, texture, ...) shares, so such an ID still
// finds the same choice. This only matches IDs: the items themselves always come from the
// catalog in the project's CRS, and an item of another CRS is never shown in their place.
//
// The catalog returns one variant of each choice: the default one, or the `_full` one only when
// the choice has no default variant in that CRS. So the same choice can be `X` in one CRS and
// `X_jgd2024_full` in the other. Within one list a choice appears once, so dropping `_full`
// as well still identifies it.
const CRS_ID_SEGMENT = /_jgd2024(?=(_full)?$)/;
const FULL_ID_SUFFIX = /_full$/;

export const toCrsAgnosticItemId = (id: string): string => id.replace(CRS_ID_SEGMENT, "");

// The key that every variant (CRS and attribute set) of one choice shares.
export const toChoiceItemId = (id: string): string =>
  toCrsAgnosticItemId(id).replace(FULL_ID_SUFFIX, "");

// Returns the item whose ID is `id`, or else the item that is the same choice as `id` in
// another CRS, or else the same choice in another attribute set.
export const findDatasetItemById = <T extends { id: string }>(
  items: readonly T[],
  id: string | undefined,
): T | undefined => {
  if (!id) return;
  const exact = items.find(item => item.id === id);
  if (exact) return exact;
  const key = toCrsAgnosticItemId(id);
  const sameCrsAgnostic = items.find(item => toCrsAgnosticItemId(item.id) === key);
  if (sameCrsAgnostic) return sameCrsAgnostic;
  const choice = toChoiceItemId(id);
  return items.find(item => toChoiceItemId(item.id) === choice);
};

// Returns the setting stored for the item, or else the one stored for the same choice under
// another variant's ID. When several match, the one whose ID differs least wins
// (CRS segment only before attribute set as well), and then the first in the list.
export const findSettingForItem = <T extends { datasetId?: string; dataId?: string }>(
  settings: readonly T[],
  datasetId: string | undefined,
  dataId: string | undefined,
): T | undefined => {
  if (!dataId) return;
  const candidates = datasetId ? settings.filter(s => s.datasetId === datasetId) : settings;
  return (
    candidates.find(s => s.dataId === dataId) ??
    candidates.find(
      s => !!s.dataId && toCrsAgnosticItemId(s.dataId) === toCrsAgnosticItemId(dataId),
    ) ??
    candidates.find(s => !!s.dataId && toChoiceItemId(s.dataId) === toChoiceItemId(dataId))
  );
};

// A PLATEAU dataset that has no item in the CRS the project shows has nothing to display, since
// the catalog never puts an item of another CRS in its place. Such datasets are left out of the
// lists when a CRS is chosen. Other datasets are kept even with no items: a generic dataset with
// no items is the description of its folder.
export const hasItemsToShow = (dataset: { __typename?: string; items: readonly unknown[] }) =>
  dataset.__typename !== "PlateauDataset" || dataset.items.length > 0;
