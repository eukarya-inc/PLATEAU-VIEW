ARG TAG=release
ARG VERSION

FROM golang:1.24-alpine AS build

RUN apk add --update --no-cache git ca-certificates build-base fuse
# RUN CGO_ENABLED=0 go install github.com/googlecloudplatform/gcsfuse@v1.4.1

WORKDIR /app

RUN --mount=type=bind,source=worker/go.mod,target=go.mod \
    --mount=type=bind,source=worker/go.sum,target=go.sum \
    --mount=type=cache,target=/go/pkg/mod \
    go mod download

COPY . /app/

RUN --mount=type=cache,target=/go/pkg/mod \
    CGO_ENABLED=0 go build -tags "${TAG}" "-ldflags=-X main.version=${VERSION} -s -w -buildid=" -trimpath -o plateauview-worker ./worker

FROM scratch

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# COPY --from=build /go/bin/gcsfuse /usr/bin/
COPY --from=build /app/plateauview-worker /app/plateauview-worker

WORKDIR /app

ENTRYPOINT [ "/app/plateauview-worker" ]
