import { FC } from "react";

import { LayerModel, LayerType } from "../../../prototypes/layers";
import {
  COLOR_SCHEME_SELECTION,
  IMAGE_SCHEME_SELECTION,
  SelectionGroup,
} from "../../../prototypes/view/states/selection";
import {
  HEATMAP_LAYER,
  MESH_CODE_LAYER,
  PEDESTRIAN_LAYER,
  SKETCH_LAYER,
  SPATIAL_ID_LAYER,
} from "../../../prototypes/view-layers";
import { useOptionalAtomValue } from "../../hooks";
import { LEGEND_DESCRIPTION_FIELD } from "../../types/fieldComponents/general";
import { ViewMarkdownViewer } from "../../ui-components/common";
import { CommonContentWrapper } from "../../ui-components/CommonContentWrapper";
import { useFindComponent } from "../../view-layers/hooks";

export interface LegendDescriptionSectionProps {
  values: (SelectionGroup & {
    type: typeof COLOR_SCHEME_SELECTION | typeof IMAGE_SCHEME_SELECTION;
  })["values"];
}

export const LegendDescriptionSection: FC<LegendDescriptionSectionProps> = ({ values }) => {
  const layer = values[0] as LayerModel<
    Exclude<
      LayerType,
      | typeof PEDESTRIAN_LAYER
      | typeof HEATMAP_LAYER
      | typeof SKETCH_LAYER
      | typeof SPATIAL_ID_LAYER
      | typeof MESH_CODE_LAYER
    >
  >;
  const legendDescriptionAtom = useFindComponent(
    layer.componentAtoms ?? [],
    LEGEND_DESCRIPTION_FIELD,
  );

  const legendDescription = useOptionalAtomValue(legendDescriptionAtom);

  return legendDescription?.preset?.description ? (
    <CommonContentWrapper>
      <ViewMarkdownViewer content={legendDescription?.preset?.description} />
    </CommonContentWrapper>
  ) : null;
};
