import { Layer } from "@reearth/core";

const value = `[
  {
    "id": "document",
    "name": "GeoJSON to CZML",
    "version": "1.0"
  },
  {
    "id": "feature_0",
    "position": {
      "cartographicDegrees": [
        30,
        0,
        0
      ]
    },
    "point": {
      "color": {
        "rgba": [
          0,
          0,
          255,
          255
        ]
      },
      "pixelSize": 10
    },
    "properties": {
      "title": 3,
      "marker-color": "#0000ff",
      "marker-size": "medium",
      "marker-symbol": "circle"
    }
  },
  {
    "id": "feature_1",
    "position": {
      "cartographicDegrees": [
        30,
        1,
        0
      ]
    },
    "point": {
      "color": {
        "rgba": [
          0,
          255,
          0,
          255
        ]
      },
      "pixelSize": 10
    },
    "properties": {
      "title": 2,
      "marker-color": "#00ff04",
      "marker-size": "medium",
      "marker-symbol": "circle"
    }
  },
  {
    "id": "feature_2",
    "position": {
      "cartographicDegrees": [
        30,
        2,
        0
      ]
    },
    "point": {
      "color": {
        "rgba": [
          255,
          0,
          0,
          255
        ]
      },
      "pixelSize": 10
    },
    "properties": {
      "title": 1,
      "marker-color": "#ff0000",
      "marker-size": "medium",
      "marker-symbol": "circle"
    }
  },
  {
    "id": "feature_3",
    "polyline": {
      "positions": {
        "cartographicDegrees": [
          31,
          2,
          0,
          32,
          2,
          0,
          33,
          2,
          0
        ]
      },
      "width": 2,
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              255,
              0,
              0,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 1,
      "stroke": "#ff0000",
      "stroke-width": 2,
      "stroke-opacity": 1
    }
  },
  {
    "id": "feature_4",
    "polyline": {
      "positions": {
        "cartographicDegrees": [
          31,
          1,
          0,
          32,
          1,
          0,
          33,
          1,
          0
        ]
      },
      "width": 2,
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              0,
              255,
              0,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 2,
      "stroke": "#00ff00",
      "stroke-width": 4,
      "stroke-opacity": 1
    }
  },
  {
    "id": "feature_5",
    "polyline": {
      "positions": {
        "cartographicDegrees": [
          31,
          0,
          0,
          32,
          0,
          0,
          33,
          0,
          0
        ]
      },
      "width": 2,
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              0,
              0,
              255,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 3,
      "stroke": "#0000ff",
      "stroke-width": 6,
      "stroke-opacity": 1
    }
  },
  {
    "id": "feature_6",
    "polygon": {
      "positions": {
        "cartographicDegrees": [
          34,
          2,
          0,
          35,
          2,
          0,
          35,
          3,
          0,
          34,
          3,
          0,
          34,
          2,
          0
        ]
      },
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              255,
              0,
              0,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 1,
      "stroke": "#ff0000",
      "stroke-width": 2,
      "stroke-opacity": 1,
      "fill": "#ff0000",
      "fill-opacity": 0.5
    }
  },
  {
    "id": "feature_7",
    "polygon": {
      "positions": {
        "cartographicDegrees": [
          34,
          1,
          0,
          35,
          1,
          0,
          35,
          2,
          0,
          34,
          2,
          0,
          34,
          1,
          0
        ]
      },
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              0,
              255,
              0,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 2,
      "stroke": "#00ff00",
      "stroke-width": 4,
      "stroke-opacity": 1,
      "fill": "#00ff00",
      "fill-opacity": 0.5
    }
  },
  {
    "id": "feature_8",
    "polygon": {
      "positions": {
        "cartographicDegrees": [
          34,
          0,
          0,
          35,
          0,
          0,
          35,
          1,
          0,
          34,
          1,
          0,
          34,
          0,
          0
        ]
      },
      "material": {
        "solidColor": {
          "color": {
            "rgba": [
              0,
              0,
              255,
              255
            ]
          }
        }
      }
    },
    "properties": {
      "title": 3,
      "stroke": "#0000ff",
      "stroke-width": 8,
      "stroke-opacity": 1,
      "fill": "#0000ff",
      "fill-opacity": 0.5
    }
  }
]`;

export const CZML_SIMPLE: Layer = {
  id: "czml_simple",
  type: "simple",
  data: {
    type: "czml",
    url: "data:text/plain;charset=UTF-8," + encodeURIComponent(value),
  },
  marker: {},
  polygon: {},
  polyline: {},
};
