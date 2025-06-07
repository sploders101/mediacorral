FROM rust:1.85.1-bullseye as rust-builder
RUN apt-get update && \
  apt-get install -y sqlite3 libudev-dev && \
  rm -rf /var/lib/apt/lists/*
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM node:23.10.0-bookworm as frontend-builder
COPY frontend /app
WORKDIR /app
RUN npm ci
RUN npm run build

FROM debian:bullseye
RUN apt-get update && \
  apt-get install -y sqlite3 libudev-dev && \
  rm -rf /var/lib/apt/lists/*
RUN mkdir /app
COPY --from=rust-builder /app/target/release/mediacorral /app/mediacorral
COPY --from=frontend-builder /app/dist /app/frontend/dist
ENTRYPOINT ["mediacorral"]
