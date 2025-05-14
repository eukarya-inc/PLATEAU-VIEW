ARG TAG=release
ARG VERSION

FROM golang:1.24-bookworm AS build

WORKDIR /app

RUN --mount=type=bind,source=go.mod,target=go.mod \
    --mount=type=bind,source=go.sum,target=go.sum \
    --mount=type=cache,target=/go/pkg/mod \
    go mod download

COPY . /app/

RUN --mount=type=cache,target=/go/pkg/mod \
    CGO_ENABLED=0 go build -tags "${TAG}" "-ldflags=-X main.version=${VERSION} -s -w -buildid=" -trimpath

FROM scratch

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /etc/mime.types /etc/mime.types
COPY --from=build /app/server /app/plateauview
COPY govpolygon/govpolygondata/ /app/govpolygondata/
COPY PlateauView3.js* reearth.yml* /app/

WORKDIR /app

CMD [ "/app/plateauview" ]
