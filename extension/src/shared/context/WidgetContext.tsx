import { ApolloProvider } from "@apollo/client";
import { Theme, ThemeOptions, ThemeProvider, createTheme } from "@mui/material";
import { merge } from "lodash-es";
import { SnackbarProvider } from "notistack";
import { FC, PropsWithChildren, useEffect, useState } from "react";

import { lightTheme, lightThemeOptions } from "../../prototypes/ui-components";
import { cityGMLClient, createCityGMLClient } from "../api/citygml";
import {
  createSettingClient,
  settingClient,
  createTemplateClient,
  templateClient,
} from "../api/clients";
import { geoClient, createGeoClient, catalogClient, createCatalogClient } from "../graphql/clients";
import { CameraPosition } from "../reearth/types";
import {
  useCityCode,
  useCityName,
  useGeoApiUrl,
  useGoogleStreetViewApiKey,
  useGsiTileUrl,
  useHideFeedback,
  useInitialPedestrianCoordinates,
  useIsCityProject,
  useMainLogo,
  useMenuLogo,
  usePlateauApiUrl,
  usePlateauGeojsonUrl,
  usePrimaryColor,
  useProjectId,
  useSiteUrl,
  useContent,
  useIsEnable,
  useStartTime,
  useFinishTime,
  useDatasetAttributesURL,
  useCityGMLApiUrl,
} from "../states/environmentVariables";

type Props = {
  inEditor?: boolean;
  // Default settings
  geoUrl?: string;
  cityGMLUrl?: string;
  gsiTileURL?: string;
  plateauUrl?: string;
  projectId?: string;
  plateauToken?: string;
  catalogUrl?: string;
  catalogURLForAdmin?: string;
  datasetAttributesURL?: string;
  googleStreetViewAPIKey?: string;
  geojsonURL?: string;
  hideFeedback?: boolean;
  // Custom settings
  projectIdForCity?: string;
  plateauTokenForCity?: string;
  cityName?: string;
  cityCode?: string;
  customPrimaryColor?: string;
  customMainLogo?: string;
  customMenuLogo?: string;
  customPedestrian?: CameraPosition;
  customSiteUrl?: string;
  // Notification setting
  isEnable?: boolean;
  content?: string;
  startTime?: string;
  finishTime?: string;
};

export const WidgetContext: FC<PropsWithChildren<Props>> = ({
  geoUrl,
  cityGMLUrl,
  gsiTileURL,
  plateauUrl,
  projectId,
  plateauToken,
  catalogUrl,
  catalogURLForAdmin,
  datasetAttributesURL,
  googleStreetViewAPIKey,
  hideFeedback,
  children,
  inEditor,
  projectIdForCity,
  plateauTokenForCity,
  cityName,
  cityCode,
  customPrimaryColor,
  customMainLogo,
  customMenuLogo,
  customPedestrian,
  customSiteUrl,
  geojsonURL,
  isEnable,
  content,
  startTime,
  finishTime,
}) => {
  const [hideFeedbackState, setHideFeedbackState] = useHideFeedback();
  useEffect(() => {
    if (hideFeedback !== undefined && hideFeedback !== hideFeedbackState) {
      setHideFeedbackState(hideFeedback);
    }
  }, [hideFeedback, hideFeedbackState, setHideFeedbackState]);

  const [plateauApiUrlState, setPlateauApiUrl] = usePlateauApiUrl();
  useEffect(() => {
    if (!plateauApiUrlState && plateauUrl) {
      setPlateauApiUrl(plateauUrl);
    }
  }, [plateauUrl, plateauApiUrlState, setPlateauApiUrl]);

  const [projectIdState, setProjectIdState] = useProjectId();
  useEffect(() => {
    if (!projectIdState && projectId) {
      setProjectIdState(projectId);
    }
  }, [projectId, projectIdState, setProjectIdState]);

  const [geoApiUrlState, setGeoApiUrlState] = useGeoApiUrl();
  useEffect(() => {
    if (!geoApiUrlState && geoUrl) {
      setGeoApiUrlState(geoUrl);
    }
  }, [geoUrl, geoApiUrlState, setGeoApiUrlState]);

  const [cityGMLApiUrlState, setCityGMLApiUrlState] = useCityGMLApiUrl();
  useEffect(() => {
    if (!cityGMLApiUrlState && cityGMLUrl) {
      setCityGMLApiUrlState(cityGMLUrl);
    }
  }, [cityGMLUrl, cityGMLApiUrlState, setCityGMLApiUrlState]);

  const [gsiTileURLState, setGSITileURLState] = useGsiTileUrl();
  useEffect(() => {
    if (!gsiTileURLState && gsiTileURL) {
      setGSITileURLState(gsiTileURL);
    }
  }, [gsiTileURL, gsiTileURLState, setGSITileURLState]);

  const [googleStreetViewAPIKeyState, setGoogleStreetViewAPIKeyState] = useGoogleStreetViewApiKey();
  useEffect(() => {
    if (!googleStreetViewAPIKeyState && googleStreetViewAPIKey) {
      setGoogleStreetViewAPIKeyState(googleStreetViewAPIKey);
    }
  }, [googleStreetViewAPIKey, googleStreetViewAPIKeyState, setGoogleStreetViewAPIKeyState]);

  // optional (custom) state
  const [cityNameState, setCityNameState] = useCityName();
  useEffect(() => {
    if (cityName && (!cityNameState || cityNameState !== cityName)) {
      setCityNameState(cityName);
    }
  }, [cityName, cityNameState, setCityNameState]);

  const [cityCodeState, setCityCodeState] = useCityCode();
  useEffect(() => {
    if (cityCode && (!cityCodeState || cityCodeState !== cityCode)) {
      setCityCodeState(cityCode);
    }
  }, [cityCode, cityCodeState, setCityCodeState]);

  const [customPrimaryColorState, setPrimaryColorState] = usePrimaryColor();
  useEffect(() => {
    if (
      customPrimaryColor &&
      (!customPrimaryColorState || customPrimaryColorState !== customPrimaryColor)
    ) {
      setPrimaryColorState(customPrimaryColor);
    }
  }, [customPrimaryColor, customPrimaryColorState, setPrimaryColorState]);

  const [customMainLogoState, setMainLogoState] = useMainLogo();
  useEffect(() => {
    if (customMainLogo && (!customMainLogoState || customMainLogoState !== customMainLogo)) {
      setMainLogoState(customMainLogo);
    }
  }, [customMainLogo, customMainLogoState, setMainLogoState]);

  const [customMenuLogoState, setMenuLogoState] = useMenuLogo();
  useEffect(() => {
    if (customMenuLogo && (!customMenuLogoState || customMenuLogoState !== customMenuLogo)) {
      setMenuLogoState(customMenuLogo);
    }
  }, [customMenuLogo, customMenuLogoState, setMenuLogoState]);

  const [customSiteUrlState, setSiteURLState] = useSiteUrl();
  useEffect(() => {
    if (customSiteUrl && (!customSiteUrlState || customSiteUrlState !== customSiteUrl)) {
      setSiteURLState(customSiteUrl);
    }
  }, [customSiteUrl, customSiteUrlState, setSiteURLState]);

  const [customPedestrianState, setInitialPededstrianCoordinatesState] =
    useInitialPedestrianCoordinates();
  useEffect(() => {
    if (
      customPedestrian &&
      (!customPedestrianState || customPedestrianState !== customPedestrian)
    ) {
      setInitialPededstrianCoordinatesState(customPedestrian);
    }
  }, [customPedestrian, customPedestrianState, setInitialPededstrianCoordinatesState]);

  const [geojsonURLState, setPlateauGeojsonUrlState] = usePlateauGeojsonUrl();
  useEffect(() => {
    if (geojsonURL && (!geojsonURLState || geojsonURLState !== geojsonURL)) {
      setPlateauGeojsonUrlState(geojsonURL);
    }
  }, [geojsonURL, geojsonURLState, setPlateauGeojsonUrlState]);

  const [datasetAttributesURLState, setDatasetAttributesURLState] = useDatasetAttributesURL();
  useEffect(() => {
    if (
      datasetAttributesURL &&
      (!datasetAttributesURLState || datasetAttributesURLState !== datasetAttributesURL)
    ) {
      setDatasetAttributesURLState(datasetAttributesURL);
    }
  }, [datasetAttributesURL, datasetAttributesURLState, setDatasetAttributesURLState]);

  // create clients
  useEffect(() => {
    if (!geoClient && geoUrl) {
      createGeoClient(geoUrl);
    }
  }, [geoUrl]);

  useEffect(() => {
    if (!cityGMLClient && cityGMLUrl) {
      createCityGMLClient(cityGMLUrl);
    }
  }, [cityGMLUrl]);

  useEffect(() => {
    const url = inEditor ? catalogURLForAdmin || catalogUrl : catalogUrl;
    if (url) {
      createCatalogClient(url, inEditor ? plateauTokenForCity || plateauToken : undefined);
    }
  }, [catalogUrl, catalogURLForAdmin, plateauToken, inEditor, plateauTokenForCity]);

  const [_, setIsCityProject] = useIsCityProject();

  useEffect(() => {
    if (!settingClient && !templateClient && plateauUrl && projectId) {
      const sidebar = `${plateauUrl}/sidebar`;
      const cityOptions = projectIdForCity
        ? { projectId: projectIdForCity, token: plateauTokenForCity }
        : undefined;
      createSettingClient(projectId, sidebar, plateauToken, cityOptions);
      createTemplateClient(projectId, sidebar, plateauToken, cityOptions);
      setIsCityProject(!!cityOptions);
    }
  }, [
    projectId,
    plateauUrl,
    plateauToken,
    projectIdForCity,
    plateauTokenForCity,
    setIsCityProject,
  ]);

  const [customTheme, setCustomTheme] = useState<Theme | undefined>(undefined);

  useEffect(() => {
    if (
      (!customTheme || customTheme.palette?.primary.main !== customPrimaryColorState) &&
      customPrimaryColorState
    ) {
      setCustomTheme(
        createTheme(
          merge<unknown, unknown, ThemeOptions>({}, lightThemeOptions, {
            palette: {
              primary: {
                main: customPrimaryColorState,
              },
            },
          }),
        ),
      );
    }
  }, [customTheme, customPrimaryColorState]);

  // notification state
  const [isEnableState, setIsEnableState] = useIsEnable();
  useEffect(() => {
    if (isEnable !== undefined && isEnable !== isEnableState) {
      setIsEnableState(isEnable);
    }
  }, [isEnable, isEnableState, setIsEnableState]);

  const [contentState, setContentState] = useContent();
  useEffect(() => {
    if (content && (!contentState || contentState !== content)) {
      setContentState(content);
    }
  }, [content, contentState, setContentState]);

  const [startTimeState, setStartTimeState] = useStartTime();
  useEffect(() => {
    if (startTime && (!startTimeState || startTimeState !== startTime)) {
      setStartTimeState(startTime);
    }
  }, [startTime, startTimeState, setStartTimeState]);

  const [finishTimeState, setFinishTimeState] = useFinishTime();
  useEffect(() => {
    if (finishTime && (!finishTimeState || finishTimeState !== finishTime)) {
      setFinishTimeState(finishTime);
    }
  }, [finishTime, finishTimeState, setFinishTimeState]);

  if (!plateauApiUrlState || !geoClient || !catalogClient || !geoApiUrlState || !gsiTileURLState) {
    return null;
  }

  return (
    <ApolloProvider client={catalogClient}>
      <ApolloProvider client={geoClient}>
        <ThemeProvider theme={customTheme ?? lightTheme}>
          <SnackbarProvider maxSnack={1}>{children}</SnackbarProvider>
        </ThemeProvider>
      </ApolloProvider>
    </ApolloProvider>
  );
};
