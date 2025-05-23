import { Cross2Icon } from "@radix-ui/react-icons";
import { Separator } from "@radix-ui/react-select";
import { FC, useState } from "react";

import { ComputedFeature, LazyLayer, SketchEditingFeature, SketchType } from "@reearth/core";

import { OptionSection } from "./common";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { SKETCH_TOOLS, TILES } from "@/constants";
import { TEST_LAYERS } from "@/testLayers";

type OptionsPanelProps = {
  currentTile: string;
  setCurrentTile: (v: string) => void;
  terrainEnabled: boolean;
  setTerrainEnabled: (v: boolean) => void;
  hideUnderground: boolean;
  setHideUnderground: (v: boolean) => void;
  activeLayerIds: string[];
  setActiveLayerIds: (v: (ids: string[]) => string[]) => void;
  sketchTool: SketchType | undefined;
  setSketchTool: (v: SketchType | undefined) => void;
  selectedLayer: LazyLayer | undefined;
  selectedFeature: ComputedFeature | undefined;
  sketchEditingFeature: SketchEditingFeature | undefined;
  sketchFeatureSelected: boolean;
  handleEditSketchFeature: () => void;
  handleCancelEditSketchFeature: () => void;
  handleApplyEditSketchFeature: () => void;
  handleDeleteSketchFeature: () => void;
  handleGetCredits: () => void;
  handleSpatialIdPick: () => void;
  spatialIdZoom: number;
  handleSpatialIdZoomChange: (v: number[]) => void;
};

const OptionsPanel: FC<OptionsPanelProps> = ({
  currentTile,
  setCurrentTile,
  terrainEnabled,
  setTerrainEnabled,
  hideUnderground,
  setHideUnderground,
  activeLayerIds,
  setActiveLayerIds,
  sketchTool,
  setSketchTool,
  sketchEditingFeature,
  sketchFeatureSelected,
  handleEditSketchFeature,
  handleCancelEditSketchFeature,
  handleApplyEditSketchFeature,
  handleDeleteSketchFeature,
  handleGetCredits,
  handleSpatialIdPick,
  spatialIdZoom,
  handleSpatialIdZoomChange,
}) => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button size="sm" className="absolute z-10 right-2 top-2" onClick={() => setOpen(true)}>
        OPTIONS
      </Button>
      <div
        className={`absolute top-0 right-0 z-20 h-full p-2 flex flex-col transition-all w-96 ${open ? "" : "translate-x-full"}`}>
        <div className="flex flex-col flex-1 gap-6 p-4 bg-white rounded-md shadow-md">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">Options</h3>
            <Button size="sm" variant="ghost" className="p-2" onClick={() => setOpen(false)}>
              <Cross2Icon className="w-4 h-4" />
            </Button>
          </div>

          <OptionSection title="Tile">
            <Select value={currentTile} onValueChange={setCurrentTile}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TILES.map(tile => (
                  <SelectItem key={tile} value={tile}>
                    {tile}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </OptionSection>

          <OptionSection title="Terrain">
            <div className="flex items-center justify-between w-full gap-2">
              <label className="text-sm font-medium leading-none opacity-70">enabled</label>
              <Switch checked={terrainEnabled} onCheckedChange={setTerrainEnabled} />
            </div>
          </OptionSection>

          <OptionSection title="Globe">
            <div className="flex items-center justify-between w-full gap-2">
              <label className="text-sm font-medium leading-none opacity-70">
                hide underground
              </label>
              <Switch checked={hideUnderground} onCheckedChange={setHideUnderground} />
            </div>
          </OptionSection>

          <OptionSection title="Layers">
            {TEST_LAYERS.map(layer => (
              <div key={layer.id} className="flex items-center gap-2">
                <Checkbox
                  checked={activeLayerIds.includes(layer.id)}
                  onCheckedChange={() => {
                    setActiveLayerIds(ids =>
                      ids.includes(layer.id)
                        ? ids.filter(id => id !== layer.id)
                        : [...ids, layer.id],
                    );
                  }}
                />
                <label className="text-sm font-medium leading-none opacity-70">{layer.id}</label>
              </div>
            ))}
          </OptionSection>

          <OptionSection title="Sketch">
            <div className="flex flex-wrap items-center gap-1">
              <Button
                size="sm"
                variant={sketchTool === undefined ? "default" : "outline"}
                onClick={() => setSketchTool(undefined)}>
                DISABLED
              </Button>
              {SKETCH_TOOLS.map(tool => (
                <Button
                  size="sm"
                  key={tool}
                  variant={sketchTool === tool ? "default" : "outline"}
                  onClick={() => setSketchTool(tool)}>
                  {tool}
                </Button>
              ))}
            </div>
            <Separator />
            <div className="flex flex-wrap items-center gap-1">
              {sketchEditingFeature !== undefined ? (
                <>
                  <Button size="sm" variant="default" onClick={handleApplyEditSketchFeature}>
                    Apply
                  </Button>
                  <Button size="sm" variant="outline" onClick={handleCancelEditSketchFeature}>
                    Cancel
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={!sketchFeatureSelected}
                    onClick={handleEditSketchFeature}>
                    Edit
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={!sketchFeatureSelected}
                    onClick={handleDeleteSketchFeature}>
                    Delete
                  </Button>
                </>
              )}
            </div>
          </OptionSection>

          <OptionSection title="Spatial ID">
            <Slider
              defaultValue={[20]}
              min={0}
              max={25}
              step={1}
              value={[spatialIdZoom]}
              onValueChange={handleSpatialIdZoomChange}
            />
            <Button size="sm" variant={"default"} onClick={handleSpatialIdPick}>
              PickSpace
            </Button>
          </OptionSection>

          <OptionSection title="Map Ref">
            <div className="flex flex-wrap items-center gap-1">
              <Button size="sm" variant="outline" onClick={handleGetCredits}>
                getCredits
              </Button>
            </div>
          </OptionSection>
        </div>
      </div>
    </>
  );
};

export default OptionsPanel;
