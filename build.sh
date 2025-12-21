#!/bin/bash

cd "$(dirname "$0")" || exit 1

rm -r ./frontend/src/generated ./backend/gen
buf generate

cd frontend
npm run build
cd -

cd backend
rm -r frontend/*
rsync -a ../frontend/dist/ ./frontend/ || exit
go build . || exit
cd -

cargo build --release || exit

rm dist/*
test -d dist || mkdir dist || exit
mv backend/backend dist/backend || exit
mv target/release/mediacorral-analysis-service dist/analysis-service || exit
mv target/release/mediacorral-drive-controller dist/drive-controller || exit
