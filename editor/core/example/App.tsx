import { CoreVisualizer } from "@reearth/core";

import OptionsPanel from "./components/OptionsPanel";
import "@/global.css";
import SelectionPanel from "./components/SelectionPanel";
import useHooks from "./hooks";

function App() {
  const {
    isReady,
    ref,
    handleMount,
    handleAPIReady,
    handleSelect,
    meta,
    currentTile,
    setCurrentTile,
    currentCamera,
    setCurrentCamera,
    terrainEnabled,
    setTerrainEnabled,
    hideUnderground,
    setHideUnderground,
    activeLayerIds,
    setActiveLayerIds,
    viewerProperty,
    layers,
    sketchTool,
    setSketchTool,
    selectedLayer,
    selectedFeature,
    sketchEditingFeature,
    sketchFeatureSelected,
    handleEditSketchFeature,
    handleCancelEditSketchFeature,
    handleApplyEditSketchFeature,
    handleDeleteSketchFeature,
    handleCreditsUpdate,
    handleGetCredits,
    handleSpatialIdPick,
    spatialIdZoom,
    handleSpatialIdZoomChange,
  } = useHooks();

  return (
    <div className="relative w-screen h-screen overflow-hidden">
      <OptionsPanel
        currentTile={currentTile}
        setCurrentTile={setCurrentTile}
        terrainEnabled={terrainEnabled}
        setTerrainEnabled={setTerrainEnabled}
        hideUnderground={hideUnderground}
        setHideUnderground={setHideUnderground}
        activeLayerIds={activeLayerIds}
        setActiveLayerIds={setActiveLayerIds}
        sketchTool={sketchTool}
        setSketchTool={setSketchTool}
        selectedLayer={selectedLayer}
        selectedFeature={selectedFeature}
        sketchEditingFeature={sketchEditingFeature}
        sketchFeatureSelected={sketchFeatureSelected}
        handleEditSketchFeature={handleEditSketchFeature}
        handleCancelEditSketchFeature={handleCancelEditSketchFeature}
        handleApplyEditSketchFeature={handleApplyEditSketchFeature}
        handleDeleteSketchFeature={handleDeleteSketchFeature}
        handleGetCredits={handleGetCredits}
        handleSpatialIdPick={handleSpatialIdPick}
        spatialIdZoom={spatialIdZoom}
        handleSpatialIdZoomChange={handleSpatialIdZoomChange}
      />
      <SelectionPanel selectedLayer={selectedLayer} selectedFeature={selectedFeature} />
      <CoreVisualizer
        ref={ref}
        ready={isReady}
        onMount={handleMount}
        onAPIReady={handleAPIReady}
        onLayerSelect={handleSelect}
        engine="cesium"
        meta={meta}
        viewerProperty={isReady ? viewerProperty : undefined}
        camera={currentCamera}
        onCameraChange={setCurrentCamera}
        onSketchTypeChangeProp={setSketchTool}
        layers={layers}
        onCreditsUpdate={handleCreditsUpdate}
      />
    </div>
  );
}

export default App;
