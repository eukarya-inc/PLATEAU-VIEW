import { useAtom } from "jotai";
import { FC } from "react";

import { ParameterList } from "../../../../prototypes/ui-components";
import { LayerDescriptionField } from "../../../types/fieldComponents/general";
import { ViewMarkdownViewer } from "../../../ui-components/common";
import { CommonContentWrapper } from "../../../ui-components/CommonContentWrapper";
import { WritableAtomForComponent } from "../../../view-layers/component";

export interface LayerLayerDescriptionFieldProps {
  atoms: WritableAtomForComponent<LayerDescriptionField>[];
}

export const LayerLayerDescriptionField: FC<LayerLayerDescriptionFieldProps> = ({ atoms }) => {
  const [component] = useAtom(atoms[0]);
  return component.preset?.description ? (
    <ParameterList>
      <CommonContentWrapper>
        <ViewMarkdownViewer content={component.preset?.description} />
      </CommonContentWrapper>
    </ParameterList>
  ) : null;
};
