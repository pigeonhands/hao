#!/bin/sh
# Builds the sample assembly into tests/fixtures/out for running the examples.
# The tests build it themselves.
set -e
cd "$(dirname "$0")"
dotnet build -c Release -o out Sample/Sample.csproj
rm -rf Sample/bin Sample/obj
