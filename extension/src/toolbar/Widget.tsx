import { AnimatePresence } from "framer-motion";
import { useAtomValue } from "jotai";
import { FC, memo } from "react";

import { LayersRenderer } from "../prototypes/layers";
import { AppFrame, LoadingScreen } from "../prototypes/ui-components";
import { AutoRotateCamera } from "../prototypes/view/containers/AutoRotateCamera";
import { Environments } from "../prototypes/view/containers/Environments";
import { HighlightedAreas } from "../prototypes/view/containers/HighlightedAreas";
import { KeyBindings } from "../prototypes/view/containers/KeyBindings";
import { PedestrianTool } from "../prototypes/view/containers/PedestrianTool";
import { ReverseGeocoding } from "../prototypes/view/containers/ReverseGeocoding";
import { ScreenSpaceCamera } from "../prototypes/view/containers/ScreenSpaceCamera";
import { ScreenSpaceSelection } from "../prototypes/view/containers/ScreenSpaceSelection";
import { SelectionCoordinator } from "../prototypes/view/containers/SelectionCoordinator";
import { SketchTool } from "../prototypes/view/containers/SketchTool";
import { ToolMachineEvents } from "../prototypes/view/containers/ToolMachineEvents";
import { readyAtom } from "../prototypes/view/states/app";
import { AppHeader } from "../prototypes/view/ui-containers/AppHeader";
import { FileDrop } from "../prototypes/view/ui-containers/FileDrop";
import { Notifications } from "../prototypes/view/ui-containers/Notifications";
import { WidgetContext } from "../shared/context/WidgetContext";
import { CameraPosition } from "../shared/reearth/types";
import { WidgetProps } from "../shared/types/widget";
import { PLATEAUVIEW_TOOLBAR_DOM_ID } from "../shared/ui-components/common/ViewClickAwayListener";
import { InitialLayers } from "../shared/view/containers/InitialLayers";
import JapanPlateauPolygon from "../shared/view/containers/JapanPlateauPolygon";
import { MeshCodeTool } from "../shared/view/containers/MeshCodeTool";
import { SpatialIdTool } from "../shared/view/containers/SpatialIdTool";
import FeedBack from "../shared/view/ui-container/Feedback";
import Help from "../shared/view/ui-container/Help";
import MyData from "../shared/view/ui-container/MyData";
import { layerComponents } from "../shared/view-layers/layerComponents";

import { InitializeApp } from "./containers/InitializeApp";
import { useAttachScreenSpaceSelection } from "./hooks/useAttachScreenSpaceSelection";
import { useSelectMeshCodeFeature } from "./hooks/useSelectMeshCodeFeature";
import { useSelectSketchFeature } from "./hooks/useSelectSketchFeature";
import { useSelectSpatialIdFeature } from "./hooks/useSelectSpatialIdFeature";

type DefaultProps = {
  geoURL?: string;
  cityGMLURL?: string;
  gsiTileURL?: string;
  arURL?: string;
  plateauURL?: string;
  plateauAccessToken?: string;
  catalogURL?: string;
  catalogURLForAdmin?: string;
  datasetAttributesURL?: string;
  projectName?: string;
  googleStreetViewAPIKey?: string;
  geojsonURL?: string;
  hideFeedback?: boolean;
};

type OptionalProps = {
  projectNameForCity?: string;
  plateauAccessTokenForCity?: string;
  cityName?: string;
  cityCode?: string;
  primaryColor?: string;
  mainLogo?: string;
  menuLogo?: string;
  pedestrian?: CameraPosition;
  siteUrl?: string;
};

type Props = WidgetProps<DefaultProps, OptionalProps>;

export const Loading: FC = () => {
  const ready = useAtomValue(readyAtom);
  return <AnimatePresence>{!ready && <LoadingScreen />}</AnimatePresence>;
};

export const Widget: FC<Props> = memo(function WidgetPresenter({ widget, inEditor }) {
  useAttachScreenSpaceSelection();
  useSelectSketchFeature();
  useSelectSpatialIdFeature();
  useSelectMeshCodeFeature();

  return (
    <div id={PLATEAUVIEW_TOOLBAR_DOM_ID}>
      <WidgetContext
        inEditor={inEditor}
        plateauUrl={widget.property.default.plateauURL}
        projectId={widget.property.default.projectName}
        plateauToken={widget.property.default.plateauAccessToken}
        catalogUrl={widget.property.default.catalogURL}
        catalogURLForAdmin={widget.property.default.catalogURLForAdmin}
        datasetAttributesURL={widget.property.default.datasetAttributesURL}
        geoUrl={widget.property.default.geoURL}
        cityGMLUrl={widget.property.default.cityGMLURL}
        gsiTileURL={widget.property.default.gsiTileURL}
        googleStreetViewAPIKey={
          widget.property.default.googleStreetViewAPIKey ||
          import.meta.env.PLATEAU_DEFAULT_GOOGLE_STREETVIEW_TOKEN
        }
        geojsonURL={widget.property.default.geojsonURL}
        hideFeedback={widget.property.default.hideFeedback}
        projectIdForCity={widget.property.optional?.projectNameForCity}
        plateauTokenForCity={widget.property.optional?.plateauAccessTokenForCity}
        cityName={widget.property.optional?.cityName}
        cityCode={widget.property.optional?.cityCode}
        customPrimaryColor={widget.property.optional?.primaryColor}
        customMainLogo={widget.property.optional?.mainLogo}
        customMenuLogo={widget.property.optional?.menuLogo}
        customPedestrian={widget.property.optional?.pedestrian}
        customSiteUrl={widget.property.optional?.siteUrl}>
        <InitializeApp />
        <AppFrame header={<AppHeader arURL={widget.property.default.arURL} />} />
        {/* TODO(ReEarth): Support initial layer loading(Splash screen) */}
        <Loading />
        {/* <Suspense>
        <SuspendUntilTilesLoaded
          initialTileCount={35}
          remainingTileCount={30}
          onComplete={handleTilesLoadComplete}> */}
        <LayersRenderer components={layerComponents} />
        {/* </SuspendUntilTilesLoaded>
      </Suspense> */}
        <Environments />
        <ToolMachineEvents />
        <Notifications />
        <InitialLayers />
        <JapanPlateauPolygon />
        <SelectionCoordinator />
        <KeyBindings />
        <ScreenSpaceSelection />
        <FileDrop />
        <ScreenSpaceCamera tiltByRightButton />
        <HighlightedAreas />
        <ReverseGeocoding />
        <PedestrianTool />
        <SketchTool />
        <SpatialIdTool />
        <MeshCodeTool />
        <MyData />
        <Help />
        <AutoRotateCamera />
        <FeedBack />
      </WidgetContext>
    </div>
  );
});
