import { Meta, StoryObj } from "@storybook/react";
import { ComponentProps, FC, useEffect, useState } from "react";

import { useVisualizer } from "./context";

import { CoreVisualizer } from ".";

export default {
  component: CoreVisualizer,
} as Meta;

type Story = StoryObj<typeof CoreVisualizer>;

export const Cesium: Story = {
  args: {
    ready: true,
    engine: "cesium",
    viewerProperty: {
      tiles: [
        {
          id: "default",
          type: "default",
        },
      ],
    },
  },
};

const Content: FC<{ ready?: boolean }> = ({ ready }) => {
  const visualizer = useVisualizer();
  useEffect(() => {
    if (!ready) return;
    visualizer.current?.engine.flyTo({
      lat: 35.683252649879606,
      lng: 139.75262379931652,
      height: 5000,
    });
    visualizer.current?.layers.add({
      type: "simple",
      data: {
        type: "geojson",
        value: {
          // GeoJSON
          type: "Feature",
          geometry: {
            coordinates: [139.75262379931652, 35.683252649879606, 1000],
            type: "Point",
          },
        },
      },
      marker: {
        imageColor: "blue",
      },
    });
  }, [visualizer, ready]);
  return null;
};

const VisualizerWrapper: FC<ComponentProps<typeof CoreVisualizer>> = props => {
  const [ready, setReady] = useState(false);
  return (
    <CoreVisualizer {...props} onMount={() => setReady(true)}>
      <Content ready={ready} />
    </CoreVisualizer>
  );
};

export const API: Story = {
  render: args => {
    return <VisualizerWrapper {...args} />;
  },
  args: {
    ready: true,
    engine: "cesium",
    viewerProperty: {
      tiles: [
        {
          id: "default",
          type: "default",
        },
      ],
    },
  },
};
