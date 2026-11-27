import { PrimitiveAtom, WritableAtom, atom } from "jotai";
import { merge } from "lodash-es";
import invariant from "tiny-invariant";

import { LayerModel, LayerType } from "../../prototypes/layers";
import { datasetTypeLayers } from "../../prototypes/view/constants/datasetTypeLayers";
import { PlateauDatasetType } from "../../prototypes/view/constants/plateau";
import { DEFAULT_SETTING_DATA_ID } from "../api/constants";
import {
  ComponentGroup,
  ComponentTemplate,
  EmphasisProperty,
  FeatureInspectorSettings,
  GeneralSetting,
  Setting,
  Template,
} from "../api/types";
import { DEFAULT_PLATEAU_SPEC_VERSION } from "../constants";
import { DatasetFragmentFragment, DatasetItem, DatasetType } from "../graphql/types/catalog";
import { REEARTH_DATA_FORMATS } from "../plateau/constants";
import { findDatasetItemById, findSettingForItem } from "../plateau/datasetItemId";
import { CameraPosition } from "../reearth/types";
import { sharedStoreAtomWrapper } from "../sharedAtoms";
import { CURRENT_COMPONENT_GROUP_ID, CURRENT_DATA_ID } from "../states/rootLayer";
import { SHARED_PROJECT_ID } from "../states/share";
import { templatesAtom } from "../states/template";
import { generateID } from "../utils/id";

import { makeComponentAtoms } from "./component";
import { ViewLayerModelParams, createViewLayer } from "./createViewLayer";

export type RootLayerForDatasetAtomParams = {
  areaCode: string;
  settings: Setting[];
  templates: Template[];
  currentDataId?: string;
  currentGroupId?: string;
  shareId?: string;
  dataset: DatasetFragmentFragment;
  hidden?: boolean;
};

export type RootLayerForDatasetParams = {
  datasetId: string;
  datasetType: DatasetType;
  subName: string | undefined;
  type: LayerType;
  title: string;
  dataList: DatasetItem[];
  settings: Setting[];
  templates: Template[];
  currentDataId: string | undefined;
  currentGroupId: string | undefined;
  shareId: string | undefined;
  shouldInitialize: boolean;
  hidden?: boolean;
  hasShareIdInParams?: boolean;
  version: number;
};

export type RootLayerForLayerAtomParams<T extends LayerType> = {
  id?: string;
  type: T;
  title?: string;
  shouldInitialize?: boolean;
  shareId?: string;
} & ViewLayerModelParams<T>;

export type RootLayerForDataset = {
  type: "dataset";
  general: GeneralSetting | undefined;
  featureInspector: FeatureInspectorSettings | undefined; // TODO: Use API definition
  layer: PrimitiveAtom<LayerModel>;
  layerName: string | undefined;
};

export type RootLayerForLayer<T extends LayerType = LayerType> = {
  id: string;
  type: "layer";
  layer: PrimitiveAtom<LayerModel<T>>;
};

export type RootLayerConfigForDataset = {
  type: "dataset";
  id: string;
  areaCode: string;
  rootLayerAtom: PrimitiveAtom<RootLayerForDataset>;
  currentGroupIdAtom: WritableAtom<string | undefined, [update: string | undefined], void>;
  currentDataIdAtom: WritableAtom<string | undefined, [update: string | undefined], void>;
  settingsAtom: WritableAtom<Setting[], [settings: Setting[]], void>;
  rawDataset: DatasetFragmentFragment;
};

export type RootLayerAtom = PrimitiveAtom<RootLayerForDataset | RootLayerForLayer>;

export type RootLayerConfigForLayer = {
  type: "layer";
  id: string;
  rootLayerAtom: PrimitiveAtom<RootLayerForLayer>;
};

export type RootLayerConfig = RootLayerConfigForDataset | RootLayerConfigForLayer;

const findComponentGroup = (
  setting: Setting | undefined,
  template: ComponentTemplate | undefined,
  currentGroupId: string | undefined,
): ComponentGroup | undefined => {
  const groups = template ? template.groups : setting?.fieldComponents?.groups;
  return currentGroupId ? groups?.find(g => g.id === currentGroupId) : groups?.[0];
};

const findComponentGroupIds = (
  setting: Setting | undefined,
  template: ComponentTemplate | undefined,
): string[] | undefined => {
  return (template ? template.groups : setting?.fieldComponents?.groups)?.map(g => g.id);
};

const findSetting = (settings: Setting[], currentDataId: string | undefined) => {
  // The setting of the item, or else the one of the same choice stored under the ID of
  // another variant (CRS or attribute set). Settings here are of one dataset already.
  const setting = findSettingForItem(settings, undefined, currentDataId);
  const defaultSetting = settings.find(s => s.dataId === DEFAULT_SETTING_DATA_ID);

  const fieldComponents = setting?.fieldComponents;
  const hasGroups = fieldComponents?.groups?.some(g => !!g.components.length);
  const hasTemplate = fieldComponents?.useTemplate && !!fieldComponents.templateId;

  const emphasisProperty = setting?.featureInspector?.emphasisProperty;
  const hasEmphasisProperty =
    !emphasisProperty?.useTemplate && emphasisProperty?.properties?.length;
  const hasEmphasisPropertyTemplate =
    emphasisProperty?.useTemplate && !!emphasisProperty?.templateId;

  const mergedSetting = merge({}, defaultSetting, setting, {
    featureInspector: {
      emphasisProperty:
        hasEmphasisProperty || hasEmphasisPropertyTemplate
          ? emphasisProperty
          : defaultSetting?.featureInspector?.emphasisProperty,
    },
    fieldComponents: hasGroups || hasTemplate ? fieldComponents : defaultSetting?.fieldComponents,
  });

  return mergedSetting as Setting | undefined;
};

const findComponentTemplate = (
  setting: Setting | undefined,
  templates: Template[],
  dataName: string | undefined,
  dataSubName: string | undefined,
): ComponentTemplate | undefined => {
  const { useTemplate, templateId, groups } = setting?.fieldComponents ?? {};

  // Should not use template if there's component settings
  if ((!useTemplate || !templateId) && groups?.some(g => !!g.components.length)) return;

  // Default template
  const templateWithName = dataName
    ? templates.find(t => [dataName, dataSubName].includes(t.name?.split("/").slice(-1)[0]))
    : undefined;

  const template =
    !useTemplate || !templateId ? templateWithName : templates.find(t => t.id === templateId);

  return template?.type === "component" ? template : undefined;
};

const findEmphasisProperties = (
  featureInspector: FeatureInspectorSettings | undefined,
  templates: Template[],
  dataName: string | undefined,
  dataSubName: string | undefined,
): EmphasisProperty[] | undefined => {
  const { useTemplate, templateId, properties } = featureInspector?.emphasisProperty ?? {};

  if ((!useTemplate || !templateId) && !!properties?.length) return properties;

  // Default template
  const templateWithName = dataName
    ? templates.find(
        t =>
          t.type === "emphasis" &&
          [dataName, dataSubName].includes(t.name?.split("/").slice(-1)[0]),
      )
    : undefined;

  const template =
    !useTemplate || !templateId ? templateWithName : templates.find(t => t.id === templateId);

  return template?.type === "emphasis" ? template.properties : undefined;
};

// An ID that names no item of the list (e.g. a choice the project's CRS does not have) falls
// back to the first item, instead of a layer with no data.
const findData = (dataList: DatasetItem[], currentDataId: string | undefined) =>
  findDatasetItemById(dataList, currentDataId) ?? dataList[0];

const createViewLayerWithComponentGroup = (
  datasetId: string,
  type: LayerType,
  title: string,
  setting: Setting | undefined,
  template: ComponentTemplate | undefined,
  data: DatasetItem | undefined,
  componentGroup: ComponentGroup | undefined,
  shareId: string | undefined,
  shouldInitialize: boolean,
  version: number,
  hidden?: boolean,
  hasShareIdInParams?: boolean,
): LayerModel => {
  invariant(type);
  return {
    ...createViewLayer({
      type,
      municipalityCode: "",
      title,
      datasetId,
      shareId,
      textured: !data?.name.includes("（テクスチャなし）"),
      shouldInitializeAtom: shouldInitialize,
      hidden: !!hidden,
    }),
    componentAtoms: makeComponentAtoms(
      datasetId,
      componentGroup?.components ?? [],
      shareId,
      shouldInitialize,
      hasShareIdInParams,
    ),
    id: datasetId,
    format: data?.format ? REEARTH_DATA_FORMATS[data.format] : undefined,
    url: data?.url,
    version,
    layers: data?.layers ?? undefined,
    cameraAtom: atom<CameraPosition | undefined>(undefined),
    componentGroups: (template ?? setting?.fieldComponents)?.groups?.map(
      g => [g.id, g.name] as [id: string, name: string],
    ),
  };
};

const createRootLayerForDataset = ({
  datasetId,
  subName,
  type,
  title,
  datasetType,
  dataList,
  settings,
  templates,
  currentDataId,
  currentGroupId,
  shareId,
  shouldInitialize,
  hidden,
  hasShareIdInParams,
  version,
}: RootLayerForDatasetParams): RootLayerForDataset => {
  const setting = findSetting(settings, currentDataId);
  const data = findData(dataList, currentDataId);
  const componentTemplate = findComponentTemplate(setting, templates, datasetType.name, subName);
  const emphasisProperties = findEmphasisProperties(
    setting?.featureInspector,
    templates,
    datasetType.name,
    subName,
  );
  const componentGroup = findComponentGroup(setting, componentTemplate, currentGroupId);

  return {
    type: "dataset",
    layerName: subName ?? datasetType.name ?? "ユースケース",
    // TODO: get settings from featureInspectorTemplate
    general: setting?.general,
    featureInspector: setting?.featureInspector
      ? {
          ...setting.featureInspector,
          emphasisProperty: {
            ...(setting.featureInspector.emphasisProperty ?? {}),
            properties: emphasisProperties,
          },
        }
      : undefined,
    layer: atom(
      createViewLayerWithComponentGroup(
        datasetId,
        type,
        title,
        setting,
        componentTemplate,
        data,
        componentGroup,
        shareId,
        shouldInitialize,
        version,
        hidden,
        hasShareIdInParams,
      ),
    ),
  };
};

export const createRootLayerForDatasetAtom = (
  params: RootLayerForDatasetAtomParams,
): RootLayerConfig => {
  const shareId = params.shareId || SHARED_PROJECT_ID;
  const dataset = params.dataset;
  const dataList = dataset.items as DatasetItem[];
  const type =
    datasetTypeLayers[dataset.type.code as PlateauDatasetType] ?? datasetTypeLayers.usecase;
  const subName =
    dataset.__typename === "PlateauDataset" ? dataset.subname ?? undefined : undefined;

  const version =
    dataset.__typename === "PlateauDataset"
      ? dataset.plateauSpecMinor.majorVersion
      : DEFAULT_PLATEAU_SPEC_VERSION;

  const initialSettings = params.settings;
  const initialTemplates = params.templates;
  // A stored ID (initial layer setting, share link) may name the item of another CRS than
  // the one this project shows; keep the ID of the item that is actually in the list.
  const initialCurrentDataId =
    findDatasetItemById(dataList, params.currentDataId)?.id ??
    dataList[0]?.id ??
    params.currentDataId;
  const initialCurrentGroupId = params.currentGroupId;
  const initialHidden = !!params.hidden;
  const rootLayerAtom = atom<RootLayerForDataset>(
    createRootLayerForDataset({
      datasetId: dataset.id,
      subName,
      type,
      title: dataset.name,
      datasetType: dataset.type as DatasetType,
      dataList,
      settings: initialSettings,
      templates: initialTemplates,
      currentDataId: initialCurrentDataId,
      currentGroupId: initialCurrentGroupId,
      shareId,
      shouldInitialize: true,
      hasShareIdInParams: !!params.shareId,
      hidden: initialHidden,
      version,
    }),
  );

  const settingsPrimitiveAtom = atom(initialSettings);

  const settingsAtom = atom(
    get => get(settingsPrimitiveAtom),
    (get, set, settings: Setting[]) => {
      const currentDataId = get(currentDataIdAtom);
      const currentGroupId = get(currentGroupIdAtom);

      set(
        rootLayerAtom,
        createRootLayerForDataset({
          datasetId: dataset.id,
          subName,
          type,
          title: dataset.name,
          datasetType: dataset.type as DatasetType,
          dataList,
          settings: settings,
          templates: get(templatesAtom),
          currentDataId: currentDataId,
          currentGroupId: currentGroupId,
          shareId,
          shouldInitialize: false,
          version,
        }),
      );
      set(settingsPrimitiveAtom, settings);
    },
  );

  const currentDataIdAtom = atom<string | undefined>(initialCurrentDataId);

  const currentGroupIdAtom = atom<string | undefined>(undefined);

  const currentDataIdAtomAtom = atom(
    get => get(currentDataIdAtom),
    (get, set, rawUpdate: string | undefined) => {
      // The shared store may restore an ID written for another CRS; see initialCurrentDataId.
      const update =
        rawUpdate === undefined
          ? undefined
          : findDatasetItemById(dataList, rawUpdate)?.id ?? dataList[0]?.id ?? rawUpdate;
      const currentDataId = get(currentDataIdAtom);
      if (currentDataId === update) return;
      // set current group id when data id changes
      const setting = findSetting(get(settingsPrimitiveAtom), update);
      const template = findComponentTemplate(
        setting,
        get(templatesAtom),
        dataset.type.name,
        subName,
      );
      const currentGroupId = findComponentGroupIds(setting, template)?.[0];
      set(currentGroupIdAtom, currentGroupId);

      set(
        rootLayerAtom,
        createRootLayerForDataset({
          datasetId: dataset.id,
          subName,
          type,
          title: dataset.name,
          datasetType: dataset.type as DatasetType,
          dataList,
          settings: get(settingsPrimitiveAtom),
          templates: get(templatesAtom),
          currentDataId: update ?? currentDataId,
          currentGroupId: currentGroupId,
          shareId,
          shouldInitialize: false,
          version,
        }),
      );
      set(currentDataIdAtom, () => update);
    },
  );

  const currentGroupIdAtomAtom = atom(
    get => get(currentGroupIdAtom),
    (get, set, update: string | undefined) => {
      const currentGroupId = get(currentGroupIdAtom);
      if (currentGroupId === update) return;

      const rootLayer = get(rootLayerAtom);
      if (rootLayer.type !== "dataset") return;

      const currentDataId = get(currentDataIdAtom);
      const setting = findSetting(get(settingsPrimitiveAtom), currentDataId);
      const data = findData(dataList, currentDataId);
      const template = findComponentTemplate(
        setting,
        get(templatesAtom),
        dataset.type.name,
        subName,
      );
      const group = findComponentGroup(setting, template, update);

      set(
        rootLayer.layer,
        createViewLayerWithComponentGroup(
          dataset.id,
          type,
          dataset.name,
          setting,
          template,
          data,
          group,
          shareId,
          false,
          version,
        ),
      );
      set(currentGroupIdAtom, () => update);
      return;
    },
  );

  const shareableCurrentDataIdName = `${dataset.id}_${CURRENT_DATA_ID}${
    shareId ? `_${shareId}` : ""
  }`;
  const shareableCurrentDataIdAtom = sharedStoreAtomWrapper(
    shareableCurrentDataIdName,
    currentDataIdAtomAtom,
    { shouldInitialize: false },
  );

  const shareableCurrentComponentGroupIdName = `${dataset.id}_${CURRENT_COMPONENT_GROUP_ID}${
    shareId ? `_${shareId}` : ""
  }`;
  const shareableCurrentGroupIdAtom = sharedStoreAtomWrapper(
    shareableCurrentComponentGroupIdName,
    currentGroupIdAtomAtom,
    { shouldInitialize: false },
  );

  return {
    type: "dataset",
    id: dataset.id,
    areaCode: params.areaCode,
    rootLayerAtom: atom(
      get => get(rootLayerAtom),
      () => {}, // readonly
    ),
    currentDataIdAtom: shareableCurrentDataIdAtom,
    currentGroupIdAtom: shareableCurrentGroupIdAtom,
    settingsAtom,
    rawDataset: dataset,
  };
};

export const createRootLayerForLayerAtom = <T extends LayerType>({
  id,
  type,
  title,
  shouldInitialize = true,
  ...props
}: RootLayerForLayerAtomParams<T>): RootLayerConfig => {
  const shareId = props.shareId || SHARED_PROJECT_ID;
  const rootLayerId = id ?? generateID();
  const rootLayer: RootLayerForLayer<T> = {
    id: rootLayerId,
    type: "layer",
    layer: atom({
      ...(createViewLayer({
        id: rootLayerId,
        type: type as LayerType,
        title: title ?? "",
        datasetId: undefined,
        municipalityCode: "",
        shouldInitializeAtom: shouldInitialize,
        ...props,
        shareId,
      }) as LayerModel<T>),
    }),
  };
  return {
    type: "layer",
    id: rootLayerId,
    rootLayerAtom: atom(
      () => rootLayer,
      () => {}, // readonly
    ) as unknown as PrimitiveAtom<RootLayerForLayer>,
  };
};
