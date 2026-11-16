FROM golang:1.24-bookworm AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY *.go ./
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /protoc-gen-bolt .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /protoc-gen-bolt /protoc-gen-bolt
ENTRYPOINT ["/protoc-gen-bolt"]
