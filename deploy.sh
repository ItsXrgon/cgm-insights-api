#!/bin/sh
# Deploy to Fly.io with git SHA as Sentry release
set -e
GIT_REV=$(git rev-parse --short=7 HEAD 2>/dev/null || echo "unknown")
fly deploy --build-arg "GIT_REV_SHORT=$GIT_REV" "$@"
