#!/usr/bin/env bash
# Build, sign and run an example on macOS so that the Secure Enclave will talk
# to it.
#
# WHY THIS EXISTS. `cargo run -p trading-example` produces an unsigned Mach-O.
# The Secure Enclave and the data-protection keychain serve signed code only,
# so the first thing the trading example does with a wallet — make a P-256
# wrapping key in the chip — comes back `-34018`, `errSecMissingEntitlement`.
# The owner's Mac reported exactly that on 2026-08-10. It is a deployment gap
# and not a defect: nothing in `session.rs` can fix it, because what is missing
# is a signature.
#
# WHAT APPLE DOCUMENTS, and it rules out the cheap version of this script:
#
#   * "Protecting a key with the Secure Enclave" is a data-protection keychain
#     feature, and "macOS builds the list of data protection keychain access
#     groups available to your program from its code signing entitlements.
#     These entitlements must be authorized by a provisioning profile."
#     — TN3137, On Mac keychain APIs and implementations.
#   * `keychain-access-groups` and `application-identifier` are *restricted*
#     entitlements: "restricted entitlements must be authorized by a
#     provisioning profile. This is an important security feature on macOS."
#     — TN3125, Inside Code Signing: Provisioning Profiles.
#   * A profile has to be embedded in the code, and "your program needs an
#     app-like bundle structure in which to embed that profile. This is
#     standard for app and app extensions but not for command-line tools."
#     — TN3137.
#
# So `codesign -s -` with a hand-written entitlements plist does not work, and
# cannot be made to work: an ad-hoc signature has no team and authorizes no
# restricted entitlement. This script therefore does the three things that
# together *do* work — wrap the binary in a `.app`, embed a real provisioning
# profile, sign with a real Apple Development identity — and refuses, saying
# what to get, at the one step no script can perform for you.
#
# The entitlements are taken from the profile rather than written here. That is
# both shorter and the only way to be sure of the team prefix: what a profile
# authorizes is exactly what it lists, so copying its own `Entitlements` across
# cannot disagree with it. A hand-composed `application-identifier` with the
# wrong ten characters produces `-34018` again, one round trip later.
#
# ponytail: this assembles a minimal `.app` of its own. `cargo ice bundle`
# (in flight) already assembles one and signs it, and when it lands the whole
# of this file collapses into two additions there — `--entitlements` on the
# codesign call and the profile copied to `Contents/embedded.provisionprofile`
# — and this script should be deleted rather than kept in step with it.
#
# UNRUN. Written on Linux against Apple's documentation. Every claim about what
# macOS does with the result is owed a confirmation from the owner's Mac; the
# two pure parts below are held by `--self-test`, which runs anywhere.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/sign-dev.sh -p <package> [-- <app arguments>...]
       scripts/sign-dev.sh --self-test

  ICE_PROVISION_PROFILE   path to a .provisionprofile (required)
  ICE_CODESIGN_IDENTITY   signing identity, when more than one is installed
USAGE
  exit 2
}

# --------------------------------------------------------------------------
# The two parts that are arithmetic rather than macOS, and are therefore the
# two parts `--self-test` holds.
# --------------------------------------------------------------------------

# The identities a development provisioning profile can be paired with, as
# `hash<tab>common name`, read from `security find-identity -v -p codesigning`
# on stdin.
#
# "Apple Development" only. A Developer ID certificate signs software for
# distribution and is not what a development profile authorizes, so offering
# one here would produce a signature the profile rejects — and the report of
# that arrives as a launch failure rather than as a message about certificates.
development_identities() {
  sed -n 's/^ *[0-9][0-9]*) *\([0-9A-F][0-9A-F]*\) *"\(Apple Development: [^"]*\)".*$/\1	\2/p'
}

# The bundle identifier an App ID names: the same string without the team.
#
# A wildcard App ID is refused rather than turned into a bundle identifier,
# because `keychain-access-groups` on a wildcard profile does not name this
# app's group and the failure it produces is the same `-34018` this script
# exists to end.
bundle_id() {
  case "$1" in
    *.\*)
      echo "the profile's App ID is the wildcard \`$1\`, which authorizes no keychain group." >&2
      echo "Register an explicit App ID with the Keychain Sharing capability and use its profile." >&2
      return 1
      ;;
    ??????????.?*) printf '%s\n' "${1#??????????.}" ;;
    *)
      echo "\`$1\` is not a team identifier followed by a bundle identifier." >&2
      return 1
      ;;
  esac
}

self_test() {
  failures=0
  check() {
    if [ "$2" = "$3" ]; then return 0; fi
    echo "FAIL $1: expected [$3], got [$2]" >&2
    failures=$((failures + 1))
  }

  # One identity among the noise `security` prints around it.
  listed=$(printf '%s\n' \
    '  1) 1A2B3C4D5E6F708192A3B4C5D6E7F80910111213 "Apple Development: owner@example.com (ABCDE12345)"' \
    '  2) 0011223344556677889900AABBCCDDEEFF001122 "Developer ID Application: Ducktape (FGHIJ67890)"' \
    '     2 valid identities found' | development_identities)
  check "one development identity is picked out" \
    "$listed" \
    "$(printf '1A2B3C4D5E6F708192A3B4C5D6E7F80910111213\tApple Development: owner@example.com (ABCDE12345)')"

  # A Developer ID on its own is not one, and the difference matters: signing
  # with it produces a profile mismatch rather than a working app.
  check "a distribution certificate is not a development identity" \
    "$(printf '%s\n' '  1) 0011223344556677889900AABBCCDDEEFF001122 "Developer ID Application: Ducktape (FGHIJ67890)"' | development_identities)" \
    ""

  check "an empty list stays empty" \
    "$(printf '%s\n' '     0 valid identities found' | development_identities)" \
    ""

  # Two is not one, and the script has to say so rather than take the first:
  # signing with the wrong one of two is a launch failure nobody can read.
  check "two identities are both reported" \
    "$(printf '%s\n' \
      '  1) AAAA "Apple Development: one@example.com (ABCDE12345)"' \
      '  2) BBBB "Apple Development: two@example.com (ABCDE12345)"' | development_identities | wc -l | tr -d ' ')" \
    "2"

  check "the team prefix comes off the App ID" \
    "$(bundle_id ABCDE12345.dev.ducktape.trading)" \
    "dev.ducktape.trading"

  check "a bundle identifier with dots in it survives whole" \
    "$(bundle_id ABCDE12345.dev.ducktape.trading.beta)" \
    "dev.ducktape.trading.beta"

  check "a wildcard App ID is refused" \
    "$(bundle_id 'ABCDE12345.*' 2>/dev/null || echo REFUSED)" \
    "REFUSED"

  check "an App ID with no team prefix is refused" \
    "$(bundle_id dev.ducktape.trading 2>/dev/null || echo REFUSED)" \
    "REFUSED"

  if [ "$failures" -ne 0 ]; then
    echo "sign-dev.sh: $failures self-test failure(s)" >&2
    exit 1
  fi
  echo "sign-dev.sh: self-test passed"
}

# --------------------------------------------------------------------------
# The rest, which is macOS and is unrun.
# --------------------------------------------------------------------------

package=""
if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit 0
fi
while [ $# -gt 0 ]; do
  case "$1" in
    -p|--package) package="${2:-}"; shift 2 || usage ;;
    --) shift; break ;;
    *) usage ;;
  esac
done
[ -n "$package" ] || usage

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(dirname -- "$script_dir")
cd "$repo_root"

[ "$(uname -s)" = "Darwin" ] || {
  echo "sign-dev.sh signs a macOS binary and only runs on macOS." >&2
  exit 1
}

profile="${ICE_PROVISION_PROFILE:-}"
if [ -z "$profile" ] || [ ! -f "$profile" ]; then
  cat >&2 <<'PROFILE'
sign-dev.sh: set ICE_PROVISION_PROFILE to a .provisionprofile file.

The Secure Enclave will not make a key for this app until the app is signed
with a `keychain-access-groups` entitlement, and macOS honours that entitlement
only when a provisioning profile authorizes it (Apple TN3125). No script can
make one; Apple issues it. Once, from developer.apple.com/account:

  1. Register an App ID — an explicit one, not a wildcard — with the
     Keychain Sharing capability enabled.
  2. Create a macOS *Development* provisioning profile for that App ID and
     this Mac, and download it.
  3. export ICE_PROVISION_PROFILE=/path/to/<name>.provisionprofile

Xcode's Signing & Capabilities tab does all three for a throwaway project and
leaves the profile in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles.
PROFILE
  exit 1
fi

identity="${ICE_CODESIGN_IDENTITY:-}"
if [ -z "$identity" ]; then
  found=$(security find-identity -v -p codesigning | development_identities || true)
  count=$(printf '%s' "$found" | grep -c . || true)
  if [ "$count" -eq 0 ]; then
    cat >&2 <<'IDENTITY'
sign-dev.sh: no "Apple Development" signing identity in your keychain.

`security find-identity -v -p codesigning` lists none. Sign in to Xcode with an
Apple ID (Settings > Accounts > Manage Certificates > + > Apple Development),
which issues one, or set ICE_CODESIGN_IDENTITY to the identity you want used.

A free Apple ID gives a Personal Team. Whether a Personal Team may authorize
the `keychain-access-groups` entitlement is not something Apple documents;
reports say it is refused, and the paid Developer Program is the documented
path. This is the one step here worth finding out cheaply before paying.
IDENTITY
    exit 1
  fi
  if [ "$count" -gt 1 ]; then
    echo "sign-dev.sh: $count development identities installed; set ICE_CODESIGN_IDENTITY to one of:" >&2
    printf '%s\n' "$found" | cut -f2 | sed 's/^/  /' >&2
    exit 1
  fi
  identity=$(printf '%s' "$found" | cut -f1)
fi

cargo build -p "$package"
binary="target/debug/$package"
[ -x "$binary" ] || { echo "sign-dev.sh: no binary at $binary" >&2; exit 1; }

work="target/sign-dev"
rm -rf "$work"
mkdir -p "$work"
decoded="$work/profile.plist"
entitlements="$work/entitlements.plist"
# A provisioning profile is CMS-signed; `security cms -D` is what unwraps it.
security cms -D -i "$profile" -o "$decoded"
# The profile's own entitlements, copied rather than composed. See the header.
plutil -extract Entitlements xml1 -o "$entitlements" "$decoded"
# Both spellings, because Apple uses both: `application-identifier` is the one
# the entitlement is documented under, and macOS profiles have been seen
# carrying `com.apple.application-identifier` instead. PlistBuddy rather than
# `plutil -extract`, whose key paths are dot-separated and cannot address the
# second name at all.
app_id=$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:application-identifier' "$decoded" 2>/dev/null \
  || /usr/libexec/PlistBuddy -c 'Print :Entitlements:com.apple.application-identifier' "$decoded" 2>/dev/null \
  || true)
if [ -z "$app_id" ]; then
  echo "sign-dev.sh: $profile names no application-identifier, so it authorizes no keychain group." >&2
  exit 1
fi
identifier=$(bundle_id "$app_id")

app="$work/$package.app"
mkdir -p "$app/Contents/MacOS"
cp "$binary" "$app/Contents/MacOS/$package"
# Verbatim, because the signature seals it and a re-encoded copy is a different
# file to the code that checks it.
cp "$profile" "$app/Contents/embedded.provisionprofile"
cat >"$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>$package</string>
	<key>CFBundleIdentifier</key><string>$identifier</string>
	<key>CFBundleName</key><string>$package</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>0.1.0</string>
	<key>CFBundleVersion</key><string>0.1.0</string>
	<!-- Without this a bundled app is drawn at 1x and upscaled, so the signed
	     build would look worse than the `cargo run` one it replaces. -->
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --sign "$identity" --entitlements "$entitlements" "$app"

cat >&2 <<REPORT
sign-dev.sh: signed $app
  identity   $identity
  App ID     $app_id
  bundle id  $identifier
  profile    $profile
  entitlements now in the signature:
REPORT
codesign -d --entitlements - "$app" >&2 || true

exec "$app/Contents/MacOS/$package" "$@"
