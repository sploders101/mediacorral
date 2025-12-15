FROM docker.io/rust:1.90.0-trixie AS rust-builder

RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get install -y libtesseract-dev libclang-19-dev protobuf-compiler \
  && rm -rf /var/cache/apt

COPY . /app
WORKDIR /app

RUN cargo build --release --locked --bin mediacorral-analysis-service

# ---

FROM docker.io/debian:trixie

RUN apt-get update \
  && DEBIAN_FRONTEND=noninteractive apt-get install -y libtesseract5 libclang-cpp19 tesseract-ocr tesseract-ocr-eng \
  && rm -rf /var/cache/apt

COPY --from=rust-builder /app/target/release/mediacorral-analysis-service /usr/bin/mediacorral-analysis-service

RUN useradd -r app
USER app
CMD ["/usr/bin/mediacorral-analysis-service"]
