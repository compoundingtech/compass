{ pkgs, compassPackage }:

# Smoke-test the built binary the way a user first meets it: it must report a
# version and print help with no catalog, no HOME, and no git checkout in sight.
#
# The version assertions are the real content here. `build.rs` embeds a build
# stamp, and its git-derived fallback cannot fire inside the Nix sandbox — so
# this pins the packaging contract: the stamp Nix injects is the one that comes
# back out, and a build that lost it would report `sourceKind: package` and fail
# here rather than silently shipping an anonymous binary.
pkgs.runCommandLocal "compass-smoke"
  {
    nativeBuildInputs = [
      compassPackage
      pkgs.jq
    ];
  }
  ''
    set -euo pipefail

    compass --help > help.txt
    test -s help.txt || { echo "compass --help printed nothing" >&2; exit 1; }
    grep -qi 'compass' help.txt

    compass version > version.txt
    test -s version.txt || { echo "compass version printed nothing" >&2; exit 1; }

    compass version --json > version.json
    jq -e '
      .sourceKind == "nix"
      and (.machineVersion | length > 0)
      and (.baseVersion | length > 0)
      and (.rev | length > 0)
    ' version.json > /dev/null || {
      echo "compass version --json did not carry the injected nix build stamp:" >&2
      cat version.json >&2
      exit 1
    }

    touch "$out"
  ''
