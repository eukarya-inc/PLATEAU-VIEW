import { gql } from "../__gen__/gql";

export const DATASETS = gql(`
query Datasets($input: DatasetsInput!, $crs: Crs) {
  datasets(input: $input) {
    ...DatasetFragment
  }
}
`);

export const DATASET_BY_ID = gql(`
query DatasetById($id: ID!, $crs: Crs) {
  node(id: $id) {
    ... on Dataset {
      ...DatasetFragment
    }
  }
}
`);

export const DATASETS_BY_IDS = gql(`
query DatasetsByIds($ids: [ID!]!, $crs: Crs) {
  nodes(ids: $ids) {
    ... on Dataset {
      ...DatasetFragment
    }
  }
}
`);
