#!/bin/sh
set -e
cd "$(dirname "$0")"
dotnet build -c Release -o out Sample/Sample.csproj
cp out/Sample.dll out/Sample.runtimeconfig.json .
rm -rf out Sample/bin Sample/obj
