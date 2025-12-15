FROM docker.io/node:24.11.1 AS frontend-builder

COPY ./frontend /app/frontend
WORKDIR /app/frontend

RUN npm ci
RUN npm run build

# ---

FROM docker.io/golang:1.25.4-trixie AS backend-builder

COPY --from=frontend-builder /app/frontend/dist/ /app/backend/frontend/
COPY . /app

WORKDIR /app/backend
RUN go build .

# ---

FROM docker.io/debian:trixie

RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    libtesseract5 tesseract-ocr-eng libclang1-19 jq \
  && rm -rf /var/cache/apt

COPY --from=backend-builder /app/backend/backend /usr/bin/mediacorral
ENV CONFIG_PATH=/etc/mediacorral.json
