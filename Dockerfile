FROM docker.io/node:24.11.1 AS frontend-builder

COPY ./frontend /app/frontend
WORKDIR /app/frontend

RUN npm ci
RUN npm run build

# ---

FROM docker.io/rust:1.90.0-trixie AS rust-builder

RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get install -y libtesseract-dev libclang-19-dev \
  && rm -rf /var/cache/apt

COPY . /app
WORKDIR /app

RUN cargo build --release --locked --bin mediacorral-analysis-cli

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

COPY --from=rust-builder /app/target/release/mediacorral-analysis-cli /usr/bin/mediacorral-analysis
COPY --from=backend-builder /app/backend/backend /usr/bin/mediacorral
ENV CONFIG_PATH=/etc/mediacorral.json
