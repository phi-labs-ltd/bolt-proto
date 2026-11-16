FROM golang:1.24-bookworm AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY *.go ./
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /bolt-proto-extensions .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /bolt-proto-extensions /bolt-proto-extensions
ENTRYPOINT ["/bolt-proto-extensions"]
