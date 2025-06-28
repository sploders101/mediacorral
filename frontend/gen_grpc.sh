#!/bin/bash

cd "$(dirname "$0")"

npx protoc \
	--ts_out src/generated/ \
	--ts_opt long_type_bigint \
	--proto_path ../mediacorral-proto/proto \
	../mediacorral-proto/proto/mediacorral/server/v1/api.proto \
	../mediacorral-proto/proto/mediacorral/server/v1/exports.proto \
	../mediacorral-proto/proto/mediacorral/common/tmdb/v1/main.proto \
	../mediacorral-proto/proto/mediacorral/drive_controller/v1/main.proto
