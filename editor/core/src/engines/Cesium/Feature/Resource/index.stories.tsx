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
        type: "czml",
        url: "/testdata/sample.czml",
      },
      resource: {},
      marker: {
        pointColor: {
          expression: {
            conditions: [["${id} === '1'", "color('red')"]],
          },
        },
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
