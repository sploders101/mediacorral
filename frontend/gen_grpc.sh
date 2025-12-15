#!/bin/bash

cd "$(dirname "$0")"

npx protoc \
	--ts_out src/generated/ \
	--ts_opt long_type_bigint \
	--proto_path ../proto \
	../proto/mediacorral/drive_controller/v1/main.proto \
	../proto/mediacorral/metadata/v1/main.proto \
	../proto/mediacorral/server/v1/api.proto \
	../proto/mediacorral/server/v1/exports.proto \
	../proto/mediacorral/server/v1/notifications.proto \
	../proto/mediacorral/server/v1/tmdb.proto
