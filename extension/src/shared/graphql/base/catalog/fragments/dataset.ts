import { gql } from "../__gen__";

export const DATASET_ITEM_FRAGMENT = gql(`
  fragment DatasetItemFragment on DatasetItem {
    id
    format
    name
    url
    layers
  }
`);

export const DATASET_FRAGMENT = gql(`
  fragment DatasetFragment on Dataset {
    id
    name
    description
    year
    groups
    openDataUrl
    prefectureId
    prefectureCode
    cityId
    cityCode
    wardId
    wardCode
    prefecture {
      name
      code
    }
    city {
      name
      code
    }
    ward {
      name
      code
    }
    type {
      id
      code
      name
      category
      order
    }
    admin
    ... on PlateauDataset {
      # The 3D Tiles of a PLATEAU dataset come in one variant per height CRS. $crs picks
      # which one the project shows; null is the server's default CRS. Only
      # PlateauDataset.items takes it, so items is selected per concrete type: arguments
      # on the Dataset interface field would conflict with this one.
      items(crs: $crs) {
        ...DatasetItemFragment
      }
      subname
      plateauSpecMinor {
        majorVersion
      }
    }
    ... on RelatedDataset {
      items {
        ...DatasetItemFragment
      }
    }
    ... on GenericDataset {
      items {
        ...DatasetItemFragment
      }
    }
  }
`);
