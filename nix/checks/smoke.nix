{ pkgs, compassPackage }:

# Smoke-test the built binary the way a user first meets it: it must report a
# version and print help without a catalog, a HOME, or a git checkout in sight.
pkgs.runCommandLocal "compass-smoke"
  {
    nativeBuildInputs = [ compassPackage ];
  }
  ''
    set -euo pipefail

    compass version > version.txt
    if ! grep -q 'compass' version.txt; then
      echo "compass version did not mention 'compass':" >&2
      cat version.txt >&2
      exit 1
    fi
    if [ ! -s version.txt ]; then
      echo "compass version printed nothing" >&2
      exit 1
    fi

    compass --help > help.txt
    if [ ! -s help.txt ]; then
      echo "compass --help printed nothing" >&2
      exit 1
    fi

    touch "$out"
  ''
