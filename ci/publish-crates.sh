#!/usr/bin/env bash
# Publish the 51Degrees workspace crates to crates.io in dependency order.
#
# Each crate is published only when its current version is not already on
# crates.io, so an ordinary push that does not bump versions is a no-op, and a
# release is just a version bump (all crates share one version) merged to main.
# After publishing a crate the script waits for the new version to appear on the
# index so the next, dependent crate resolves it.
#
# `fodid` and `fiftyone-fodid-cloud` depend on `owid`, which is consumed from
# crates.io (the workspace `owid` dependency is a published version, not git), so
# they are publishable and included below in dependency order.
set -euo pipefail

# Publishing needs a crates.io token. When it is absent (for example a normal
# push before the secret is configured) skip cleanly rather than fail, so the
# workflow only acts once a token is in place.
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "CARGO_REGISTRY_TOKEN is not set; skipping crates.io publish."
  exit 0
fi

# Dependency order: a crate appears after every workspace crate it depends on.
CRATES=(
  fiftyone-common-sys
  fiftyone-device-detection-sys
  fiftyone-ip-intelligence-sys
  fiftyone-pipeline-core
  fodid
  fiftyone-caching
  fiftyone-native
  fiftyone-pipeline-engines
  fiftyone-pipeline-engines-fiftyone
  fiftyone-cloud-request-engine
  fiftyone-device-detection-shared
  fiftyone-fodid-cloud
  fiftyone-ip-intelligence-shared
  fiftyone-json-builder
  fiftyone-device-detection-cloud
  fiftyone-device-detection-onpremise
  fiftyone-ip-intelligence-cloud
  fiftyone-ip-intelligence-onpremise
  fiftyone-javascript-builder
  fiftyone-pipeline-web
  fiftyone-pipeline-web-axum
  fiftyone-ip-intelligence
  fiftyone-device-detection
)

UA="51degrees-rust-publish (support@51degrees.com)"

# True when the given crate version is already on crates.io. The API returns 200
# for a published version and 404 otherwise; a User-Agent is required.
is_published() {
  curl -sf -H "User-Agent: $UA" \
    "https://crates.io/api/v1/crates/$1/$2" >/dev/null 2>&1
}

crate_version() {
  cargo metadata --no-deps --format-version 1 \
    | jq -r ".packages[] | select(.name==\"$1\") | .version"
}

# Publish one crate, waiting out crates.io's new-crate rate limit. crates.io
# limits how many brand-new crate names one account may publish in a short
# window and replies 429 with a "try again after <date>" once the burst is
# spent. On a 429 this sleeps until that time (plus a margin) and retries the
# same crate; any other failure is fatal. Once every crate has been published
# once, later version bumps are not new-crate publishes and are not throttled.
publish_crate() {
  local crate="$1" attempt out when target now wait
  for attempt in $(seq 1 40); do
    if out="$(cargo publish -p "$crate" 2>&1)"; then
      echo "$out"
      return 0
    fi
    echo "$out"
    if echo "$out" | grep -qiE "429|too many requests|too many .* crates"; then
      when="$(echo "$out" | grep -oiE "try again after [^.]*" \
        | head -1 | sed -E 's/[Tt]ry again after //')"
      target="$(date -u -d "$when" +%s 2>/dev/null || echo "")"
      now="$(date -u +%s)"
      if [ -n "$target" ] && [ "$target" -gt "$now" ]; then
        wait=$(( target - now + 15 ))
      else
        wait=130
      fi
      echo ">> rate limited; sleeping ${wait}s then retrying $crate (attempt $attempt)"
      sleep "$wait"
      continue
    fi
    echo ">> $crate failed to publish for a non-rate-limit reason; stopping."
    return 1
  done
  echo ">> gave up on $crate after $attempt attempts."
  return 1
}

for crate in "${CRATES[@]}"; do
  version="$(crate_version "$crate")"
  if [ -z "$version" ]; then
    echo "ERROR: could not resolve a version for $crate"
    exit 1
  fi
  if is_published "$crate" "$version"; then
    echo "== $crate $version already on crates.io; skipping."
    continue
  fi
  echo "== publishing $crate $version"
  publish_crate "$crate"
  # Wait for the new version to be queryable so the next dependent crate's
  # verification build can resolve it from the index.
  for attempt in $(seq 1 30); do
    if is_published "$crate" "$version"; then
      echo "   $crate $version is indexed."
      break
    fi
    echo "   waiting for the index to pick up $crate $version ($attempt/30)..."
    sleep 10
  done
done

echo "Done."
