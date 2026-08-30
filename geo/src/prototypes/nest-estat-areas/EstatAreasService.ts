import { CollectionReference, type Query, type QuerySnapshot } from "@google-cloud/firestore";
import { Inject, Injectable } from "@nestjs/common";
import { isNotNullish } from "@prototypes/type-helpers";
import { plainToInstance } from "class-transformer";
import { uniqBy } from "lodash";
import invariant from "tiny-invariant";

import { EstatArea } from "./dto/EstatArea";
import { EstatAreaDocument } from "./dto/EstatAreaDocument";
import { EstatAreaGeometry } from "./dto/EstatAreaGeometry";
import { unpackGeometry } from "./helpers/packGeometry";

const areaPropertyKeys = ["PREF_NAME", "GST_NAME", "CSS_NAME", "S_NAME", "PREF", "CITY"] as const;

const selectFields = [...areaPropertyKeys.map(field => `properties.${field}`), "bbox"];

type AreaQuerySnapshot = QuerySnapshot<{
  properties: Pick<EstatAreaDocument["properties"], (typeof areaPropertyKeys)[number]>;
  bbox: EstatAreaDocument["bbox"];
}>;

function createAreas(snapshot: AreaQuerySnapshot, searchTokens?: readonly string[]): EstatArea[] {
  const result = snapshot.docs.map(doc => {
    const data = doc.data();
    const props = data.properties;
    const addressComponents = [
      props.PREF_NAME,
      props.GST_NAME,
      props.CSS_NAME,
      props.S_NAME,
    ].filter(isNotNullish);
    return plainToInstance(EstatArea, {
      id: doc.id,
      prefectureCode: props.PREF,
      municipalityCode: `${props.PREF}${props.CITY}`,
      name: props.S_NAME,
      address: addressComponents.join(""),
      addressComponents,
      bbox: data.bbox,
    } satisfies EstatArea);
  });
  if (searchTokens == null || searchTokens.length === 0) {
    return result;
  }
  return result.filter(result =>
    searchTokens.every(token => result.addressComponents.includes(token)),
  );
}

// Cap the client-controlled `limit` argument. Without this the resolver
// forwards the caller's value straight into `.limit(limit * 2)` and
// `uniqBy([...result, ...createAreas(...)])`, materializing every returned
// document in memory — a single `limit: 1_000_000` request would pull up to
// ~2M docs onto the geo Cloud Run service and OOM the instance.
const ESTAT_AREAS_MAX_LIMIT = 1000;
const ESTAT_AREAS_DEFAULT_LIMIT = 100;

const searchFields = ["shortAddress", "middleAddress", "fullAddress"];
const compoundSearchFields = [
  ["properties.S_NAME", "properties.CSS_NAME", "properties.GST_NAME", "properties.PREF_NAME"],
  ["properties.S_NAME", "properties.CSS_NAME", "properties.GST_NAME"],
  ["properties.S_NAME", "properties.CSS_NAME", "properties.PREF_NAME"],
  ["properties.S_NAME", "properties.GST_NAME", "properties.PREF_NAME"],
  ["properties.S_NAME", "properties.CSS_NAME"],
  ["properties.S_NAME", "properties.GST_NAME"],
  ["properties.S_NAME", "properties.PREF_NAME"],
  ["properties.CSS_NAME", "properties.GST_NAME", "properties.PREF_NAME"],
  ["properties.CSS_NAME", "properties.PREF_NAME"],
  ["properties.GST_NAME", "properties.PREF_NAME"],
  ["properties.S_NAME"],
  ["properties.CSS_NAME"],
  ["properties.GST_NAME"],
  ["properties.PREF_NAME"],
];

@Injectable()
export class EstatAreasService {
  constructor(
    @Inject(EstatAreaDocument)
    private readonly areaCollection: CollectionReference<EstatAreaDocument>,
  ) {}

  async findAll(params: { searchTokens: readonly string[]; limit?: number }): Promise<EstatArea[]> {
    const requested = params.limit ?? ESTAT_AREAS_DEFAULT_LIMIT;
    // Clamp to [1, ESTAT_AREAS_MAX_LIMIT]. Non-finite / non-positive values
    // fall back to the default; sub-1 positive fractions (e.g. `0.5` → floor
    // = 0) would otherwise sneak through `> 0` and produce a `.limit(0)`
    // query that always returns empty — floor first, then re-check the
    // effective integer against the same lower bound.
    let limit = ESTAT_AREAS_DEFAULT_LIMIT;
    if (Number.isFinite(requested) && requested > 0) {
      const floored = Math.floor(requested);
      if (floored >= 1) {
        limit = Math.min(floored, ESTAT_AREAS_MAX_LIMIT);
      }
    }

    let result: EstatArea[] = [];
    for (const fields of compoundSearchFields) {
      let disjunctionCount = 1;
      let query: Query | typeof this.areaCollection = this.areaCollection;
      for (const field of fields) {
        disjunctionCount *= params.searchTokens.length;
        if (disjunctionCount > 30) {
          break;
        }
        query = query.where(field, "in", params.searchTokens);
      }
      const snapshot = (await query
        .orderBy("properties.SETAI", "desc")
        .limit(limit * 2) // Double this because some will be filtered out.
        .select(...selectFields)
        .get()) as AreaQuerySnapshot;
      result = uniqBy([...result, ...createAreas(snapshot, params.searchTokens)], "id");
      if (result.length >= limit) {
        return result.slice(0, limit);
      }
    }
    if (result.length > 0) {
      return result;
    }

    const [searchToken] = [...params.searchTokens].sort((a, b) => b.length - a.length);
    for (const field of searchFields) {
      const snapshot = (await this.areaCollection
        .where(field, ">=", searchToken)
        .where(field, "<=", `${searchToken}\uf8ff`)
        .limit(limit)
        .select(...selectFields)
        .get()) as AreaQuerySnapshot;
      if (!snapshot.empty) {
        return createAreas(snapshot);
      }
    }
    return [];
  }

  async findGeometry(params: { areaId: string }): Promise<EstatAreaGeometry | undefined> {
    const doc = await this.areaCollection.doc(params.areaId).get();
    if (!doc.exists) {
      return;
    }
    const data = doc.data();
    invariant(data != null);
    return plainToInstance(EstatAreaGeometry, {
      id: doc.id,
      // @ts-expect-error Coerce to JSON type
      geometry: unpackGeometry(data.geometry),
    } satisfies EstatAreaGeometry);
  }
}
