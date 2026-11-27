/* eslint-disable */
import * as types from './graphql';
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 */
const documents = {
    "\n  fragment DatasetItemFragment on DatasetItem {\n    id\n    format\n    name\n    url\n    layers\n  }\n": types.DatasetItemFragmentFragmentDoc,
    "\n  fragment DatasetFragment on Dataset {\n    id\n    name\n    description\n    year\n    groups\n    openDataUrl\n    prefectureId\n    prefectureCode\n    cityId\n    cityCode\n    wardId\n    wardCode\n    prefecture {\n      name\n      code\n    }\n    city {\n      name\n      code\n    }\n    ward {\n      name\n      code\n    }\n    type {\n      id\n      code\n      name\n      category\n      order\n    }\n    admin\n    ... on PlateauDataset {\n      # The 3D Tiles of a PLATEAU dataset come in one variant per height CRS. $crs picks\n      # which one the project shows; null is the server's default CRS. Only\n      # PlateauDataset.items takes it, so items is selected per concrete type: arguments\n      # on the Dataset interface field would conflict with this one.\n      items(crs: $crs) {\n        ...DatasetItemFragment\n      }\n      subname\n      plateauSpecMinor {\n        majorVersion\n      }\n    }\n    ... on RelatedDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n    ... on GenericDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n  }\n": types.DatasetFragmentFragmentDoc,
    "\nquery Areas($input: AreasInput!) {\n  areas(input: $input) {\n    id\n    code\n    name\n  }\n}\n": types.AreasDocument,
    "\nquery AreaDatasets($code: AreaCode!, $input: DatasetsInput!, $crs: Crs) {\n  area(code: $code) {\n    id\n    code\n    name\n    datasets(input: $input) {\n      ...DatasetFragment\n    }\n  }\n}\n": types.AreaDatasetsDocument,
    "\nquery Datasets($input: DatasetsInput!, $crs: Crs) {\n  datasets(input: $input) {\n    ...DatasetFragment\n  }\n}\n": types.DatasetsDocument,
    "\nquery DatasetById($id: ID!, $crs: Crs) {\n  node(id: $id) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n": types.DatasetByIdDocument,
    "\nquery DatasetsByIds($ids: [ID!]!, $crs: Crs) {\n  nodes(ids: $ids) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n": types.DatasetsByIdsDocument,
    "\nquery DatasetTypes($input: DatasetTypesInput) {\n  datasetTypes(input: $input) {\n    id\n    name\n    code\n    order\n  }\n}\n": types.DatasetTypesDocument,
    "\nquery PlateauSpecs {\n  plateauSpecs {\n    majorVersion\n  }\n}\n": types.PlateauSpecsDocument,
};

/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = gql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function gql(source: string): unknown;

/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\n  fragment DatasetItemFragment on DatasetItem {\n    id\n    format\n    name\n    url\n    layers\n  }\n"): (typeof documents)["\n  fragment DatasetItemFragment on DatasetItem {\n    id\n    format\n    name\n    url\n    layers\n  }\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\n  fragment DatasetFragment on Dataset {\n    id\n    name\n    description\n    year\n    groups\n    openDataUrl\n    prefectureId\n    prefectureCode\n    cityId\n    cityCode\n    wardId\n    wardCode\n    prefecture {\n      name\n      code\n    }\n    city {\n      name\n      code\n    }\n    ward {\n      name\n      code\n    }\n    type {\n      id\n      code\n      name\n      category\n      order\n    }\n    admin\n    ... on PlateauDataset {\n      # The 3D Tiles of a PLATEAU dataset come in one variant per height CRS. $crs picks\n      # which one the project shows; null is the server's default CRS. Only\n      # PlateauDataset.items takes it, so items is selected per concrete type: arguments\n      # on the Dataset interface field would conflict with this one.\n      items(crs: $crs) {\n        ...DatasetItemFragment\n      }\n      subname\n      plateauSpecMinor {\n        majorVersion\n      }\n    }\n    ... on RelatedDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n    ... on GenericDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n  }\n"): (typeof documents)["\n  fragment DatasetFragment on Dataset {\n    id\n    name\n    description\n    year\n    groups\n    openDataUrl\n    prefectureId\n    prefectureCode\n    cityId\n    cityCode\n    wardId\n    wardCode\n    prefecture {\n      name\n      code\n    }\n    city {\n      name\n      code\n    }\n    ward {\n      name\n      code\n    }\n    type {\n      id\n      code\n      name\n      category\n      order\n    }\n    admin\n    ... on PlateauDataset {\n      # The 3D Tiles of a PLATEAU dataset come in one variant per height CRS. $crs picks\n      # which one the project shows; null is the server's default CRS. Only\n      # PlateauDataset.items takes it, so items is selected per concrete type: arguments\n      # on the Dataset interface field would conflict with this one.\n      items(crs: $crs) {\n        ...DatasetItemFragment\n      }\n      subname\n      plateauSpecMinor {\n        majorVersion\n      }\n    }\n    ... on RelatedDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n    ... on GenericDataset {\n      items {\n        ...DatasetItemFragment\n      }\n    }\n  }\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery Areas($input: AreasInput!) {\n  areas(input: $input) {\n    id\n    code\n    name\n  }\n}\n"): (typeof documents)["\nquery Areas($input: AreasInput!) {\n  areas(input: $input) {\n    id\n    code\n    name\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery AreaDatasets($code: AreaCode!, $input: DatasetsInput!, $crs: Crs) {\n  area(code: $code) {\n    id\n    code\n    name\n    datasets(input: $input) {\n      ...DatasetFragment\n    }\n  }\n}\n"): (typeof documents)["\nquery AreaDatasets($code: AreaCode!, $input: DatasetsInput!, $crs: Crs) {\n  area(code: $code) {\n    id\n    code\n    name\n    datasets(input: $input) {\n      ...DatasetFragment\n    }\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery Datasets($input: DatasetsInput!, $crs: Crs) {\n  datasets(input: $input) {\n    ...DatasetFragment\n  }\n}\n"): (typeof documents)["\nquery Datasets($input: DatasetsInput!, $crs: Crs) {\n  datasets(input: $input) {\n    ...DatasetFragment\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery DatasetById($id: ID!, $crs: Crs) {\n  node(id: $id) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n"): (typeof documents)["\nquery DatasetById($id: ID!, $crs: Crs) {\n  node(id: $id) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery DatasetsByIds($ids: [ID!]!, $crs: Crs) {\n  nodes(ids: $ids) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n"): (typeof documents)["\nquery DatasetsByIds($ids: [ID!]!, $crs: Crs) {\n  nodes(ids: $ids) {\n    ... on Dataset {\n      ...DatasetFragment\n    }\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery DatasetTypes($input: DatasetTypesInput) {\n  datasetTypes(input: $input) {\n    id\n    name\n    code\n    order\n  }\n}\n"): (typeof documents)["\nquery DatasetTypes($input: DatasetTypesInput) {\n  datasetTypes(input: $input) {\n    id\n    name\n    code\n    order\n  }\n}\n"];
/**
 * The gql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function gql(source: "\nquery PlateauSpecs {\n  plateauSpecs {\n    majorVersion\n  }\n}\n"): (typeof documents)["\nquery PlateauSpecs {\n  plateauSpecs {\n    majorVersion\n  }\n}\n"];

export function gql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;