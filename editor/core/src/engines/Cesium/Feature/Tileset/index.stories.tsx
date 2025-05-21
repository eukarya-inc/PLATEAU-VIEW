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
        type: "3dtiles",
        url: `https://example.com/3dtiles.json`, // You need to set URL.
      },
      ["3dtiles"]: {
        color: "red",
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
