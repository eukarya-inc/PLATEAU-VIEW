import { Meta, StoryFn } from "@storybook/react";

import { engine } from "../..";
import { Map, Props } from "../../../../Map";

export default {
  component: Map,
  parameters: { actions: { argTypesRegex: "^on.*" } },
} as Meta;

const Template: StoryFn<Props> = args => <Map {...args} />;

export const Default = Template.bind([]);
Default.args = {
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
        type: "geojson",
        value: {
          type: "Feature",
          geometry: {
            type: "Point",
            coordinates: [0, 0, 1000],
          },
        },
      },
      ellipse: {
        radius: 1000,
        fillColor: "#FF0000",
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
