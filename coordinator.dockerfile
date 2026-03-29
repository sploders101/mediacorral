FROM --platform=$BUILDPLATFORM docker.io/node:24.11.1 AS frontend-builder

COPY ./frontend /app/frontend
WORKDIR /app/frontend

RUN npm ci
RUN npm run build

# ---

FROM --platform=$BUILDPLATFORM docker.io/golang:1.26.1-alpine AS backend-builder

ARG TARGETOS
ARG TARGETARCH

COPY ./backend /app/backend
COPY --from=frontend-builder /app/frontend/dist/ /app/backend/frontend/

WORKDIR /app/backend
RUN GOOS=${TARGETOS} GOARCH=${TARGETARCH} go build .

# ---

FROM gcr.io/distroless/static-debian12

COPY --from=backend-builder /app/backend/backend /bin/mediacorral
ENV CONFIG_PATH=/config/mediacorral.json
ENTRYPOINT ["/bin/mediacorral"]
