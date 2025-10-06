#!/usr/bin/env sh

podman compose down && podman compose build && podman compose up -d