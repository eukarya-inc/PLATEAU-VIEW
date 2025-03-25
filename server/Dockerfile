ARG TAG=release,draco
ARG VERSION

FROM golang:1.24-bullseye AS build

WORKDIR /app

RUN --mount=type=bind,source=go.mod,target=go.mod \
    --mount=type=bind,source=go.sum,target=go.sum \
    --mount=type=cache,target=/go/pkg/mod \
    go mod download

COPY . /app/

RUN --mount=type=cache,target=/go/pkg/mod \
    go build -tags "${TAG}" "-ldflags=-X main.version=${VERSION} -s -w -buildid=" -trimpath

FROM debian:bullseye

RUN apt-get update && apt-get install -y ca-certificates && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/server /app/reearth-plateauview
COPY govpolygon/govpolygondata/ /app/govpolygondata/
COPY PlateauView3.js* reearth.yml* /app/

WORKDIR /app

CMD [ "/app/reearth-plateauview" ]
