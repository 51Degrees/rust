#!/usr/bin/env bash
# One-time setup: register this repository's publish workflow as a Trusted
# Publisher on crates.io for every published crate, so the publish workflow can
# authenticate with a short-lived OIDC token instead of a stored API token.
# See https://crates.io/docs/trusted-publishing.
#
# Run once, with a crates.io API token in CRATES_IO_TOKEN that belongs to an
# owner of the crates:
#
#   CRATES_IO_TOKEN=cio_xxx bash ci/setup-trusted-publishing.sh
#
# Idempotent: a crate that already has this exact trusted publisher is skipped.
# After this has run, the publish workflow no longer needs CARGO_REGISTRY_TOKEN.
set -uo pipefail

: "${CRATES_IO_TOKEN:?Set CRATES_IO_TOKEN to a crates.io API token that owns the crates}"

OWNER="51Degrees"
REPO="rust"
WORKFLOW="publish.yml"
API="https://crates.io/api/v1/trusted_publishing/github_configs"
UA="51degrees-trusted-publishing-setup (support@51degrees.com)"

CRATES=(
  fiftyone-common-sys fiftyone-device-detection-sys fiftyone-ip-intelligence-sys
  fiftyone-pipeline-core fodid fiftyone-caching fiftyone-native
  fiftyone-pipeline-engines fiftyone-pipeline-engines-fiftyone
  fiftyone-cloud-request-engine fiftyone-device-detection-shared
  fiftyone-fodid-cloud fiftyone-ip-intelligence-shared fiftyone-json-builder
  fiftyone-device-detection-cloud fiftyone-device-detection-onpremise
  fiftyone-ip-intelligence-cloud fiftyone-ip-intelligence-onpremise
  fiftyone-javascript-builder fiftyone-pipeline-web fiftyone-pipeline-web-axum
  fiftyone-ip-intelligence fiftyone-device-detection
)

already_configured() {
  # True when a github config for this owner/repo/workflow already exists.
  curl -fsS -H "User-Agent: $UA" -H "Authorization: $CRATES_IO_TOKEN" \
    "$API?crate=$1" 2>/dev/null \
    | grep -q "\"repository_owner\":\"$OWNER\"" 2>/dev/null \
    && curl -fsS -H "User-Agent: $UA" -H "Authorization: $CRATES_IO_TOKEN" \
       "$API?crate=$1" 2>/dev/null | grep -q "\"workflow_filename\":\"$WORKFLOW\""
}

failures=0
for crate in "${CRATES[@]}"; do
  if already_configured "$crate"; then
    echo "= $crate: trusted publisher already configured, skipping"
    continue
  fi
  body=$(printf '{"github_config":{"crate":"%s","repository_owner":"%s","repository_name":"%s","workflow_filename":"%s","environment":null}}' \
    "$crate" "$OWNER" "$REPO" "$WORKFLOW")
  resp=$(curl -sS -X POST -H "User-Agent: $UA" -H "Authorization: $CRATES_IO_TOKEN" \
    -H "Content-Type: application/json" -d "$body" "$API" 2>&1)
  if echo "$resp" | grep -q '"github_config"'; then
    echo "+ $crate: trusted publisher registered"
  else
    echo "! $crate: FAILED -> $resp"
    failures=$((failures + 1))
  fi
done

echo
if [ "$failures" -gt 0 ]; then
  echo "Trusted publishing setup finished with $failures failure(s)."
  exit 1
fi
echo "Trusted publishing configured for all crates."
