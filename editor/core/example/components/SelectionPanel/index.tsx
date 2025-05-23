import { Cross2Icon } from "@radix-ui/react-icons";
import { FC, useState } from "react";

import { ComputedFeature, LazyLayer } from "@reearth/core";

import { OptionSection } from "../OptionsPanel/common";

import { Button } from "@/components/ui/button";

type SelectionPanelProps = {
  selectedLayer: LazyLayer | undefined;
  selectedFeature: ComputedFeature | undefined;
};

const SelectionPanel: FC<SelectionPanelProps> = ({ selectedLayer, selectedFeature }) => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button size="sm" className="absolute z-10 left-2 top-2" onClick={() => setOpen(true)}>
        Selections
      </Button>
      <div
        className={`absolute top-0 left-0 z-20 p-2 transition-all  w-96 ${open ? "" : "-translate-x-full"}`}>
        <div className="flex flex-col gap-6 p-4 bg-white rounded-md shadow-md">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">Selections</h3>
            <Button size="sm" variant="ghost" className="p-2" onClick={() => setOpen(false)}>
              <Cross2Icon className="w-4 h-4" />
            </Button>
          </div>

          <OptionSection title="SelectedLayer">
            <span className="text-sm break-all">
              {selectedLayer ? JSON.stringify(selectedLayer) : "undefined"}
            </span>
          </OptionSection>

          <OptionSection title="SelectedFeature">
            <span className="text-sm break-all">
              {selectedFeature ? JSON.stringify(selectedFeature) : "undefined"}
            </span>
          </OptionSection>
        </div>
      </div>
    </>
  );
};

export default SelectionPanel;
