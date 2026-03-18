import { Meta, StoryFn } from "@storybook/react";

import { engine } from "../..";
import { Map, Props } from "../../../../Map";

export default {
  component: Map,
  parameters: { actions: { argTypesRegex: "^on.*" } },
} as Meta;

const Template: StoryFn<Props> = args => <Map {...args} />;

export const WMS = Template.bind([]);
WMS.args = {
  engine: "cesium",
  engines: {
    cesium: engine,
  },
  ready: true,
  layers: [
    {
      id: "l",
      type: "simple",
      data: {
        type: "wms",
        url: "https://gibs.earthdata.nasa.gov/wms/epsg3857/best/wms.cgi",
        layers: "IMERG_Precipitation_Rate",
      },
      raster: {
        maximumLevel: 100,
      },
    },
  ],
  property: {
    tiles: [
      {
        id: "default",
        type: "default",
      },
    ],
  },
};

export const MVT = Template.bind([]);
MVT.args = {
  engine: "cesium",
  engines: {
    cesium: engine,
  },
  ready: true,
  layers: [
    {
      id: "l",
      type: "simple",
      data: {
        type: "mvt",
        url: "https://example.com/exmaple.mvt", // You need to set MVT URL.
        layers: "road",
      },
      polygon: {
        fillColor: "white",
        strokeColor: "white",
        strokeWidth: 1,
        lineJoin: "round",
      },
      raster: {},
    },
  ],
  property: {
    tiles: [
      {
        id: "default",
        type: "default",
      },
    ],
  },
};
