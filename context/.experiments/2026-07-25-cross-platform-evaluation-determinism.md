# Cross-platform evaluation determinism

Date: 2026-07-25

Validates the property that a plan's identity is the same on every platform —
the basis for "same source produces the same version everywhere", which several
requirements rest on and which had been an inference rather than a measurement.

## Hypothesis

Evaluating a plan produces byte-identical output on x86_64-linux,
aarch64-linux, and aarch64-darwin, because a plan is pure data construction and
the only platform-variable surface identified in earlier research —
libc-delegated transcendental Math — is not reached by a plan and is removed
from the evaluation global regardless.

## Method

Plan identity is the hash of the evaluated value's canonical serialization, and
transpilation does not enter identity (0011 Amendment 1), so the only
platform-variable component is the JavaScript engine. The engine is QuickJS-ng
(0011), whose standalone interpreter `qjs` is in nixpkgs, so the engine could be
exercised directly without building the full pipeline.

A probe constructed a representative evaluated plan — nested objects and arrays,
Unicode and combining-character strings, integer and non-integer arithmetic, and
integer-like object keys to exercise the key-ordering rule — and printed its
canonical serialization. As a negative control it also printed the transcendental
Math functions the design removes from the global, to see whether they were in
fact a source of cross-platform divergence.

The identical probe ran under `nix shell nixpkgs#quickjs-ng -c qjs` on three
machines spanning both dimensions that could matter, architecture and libc:

| Machine | Platform |
| --- | --- |
| local | x86_64-linux (glibc) |
| dev4 | aarch64-linux (glibc) |
| mbp2025 | aarch64-darwin (Apple libm) |

The serialized-plan line was hashed on each and compared.

## Result

The canonical plan serialization was **byte-identical on all three platforms**
(SHA-256 `5160e9b3…`). The hypothesis holds: a plan's identity does not depend on
the platform it is evaluated on.

The negative control was more interesting than expected. Every transcendental —
`sin`, `cos`, `exp`, `log`, `pow`, `tanh` — was **also identical across all
three**, including on Apple libm. The predicted glibc-vs-Apple-libm divergence
did not manifest for these inputs on QuickJS-ng 0.14.0.

## Interpretation

The load-bearing conclusion is unaffected and now measured rather than assumed:
plan identity is platform-independent, because a plan constructs data and touches
none of the platform-variable surface.

The control result is a genuine discrepancy with the earlier research, which read
QuickJS-ng master delegating every transcendental straight to libc and cited
1-ULP glibc/Apple disagreements. Two readings are consistent with both: the
disagreements are input-specific and these inputs happen to agree, or 0.14.0
differs from the master source that was read. This experiment does not
distinguish them, and does **not** establish that transcendentals agree across
platforms in general — only that they did here.

The Math lock therefore stands as insurance rather than a demonstrated-necessary
fix: primary sources still show some inputs diverge, a plan has no reason to call
these functions, and removing them costs three lines. What this experiment
establishes is that the lock is not load-bearing for plan identity — identity is
safe because plans do not do arithmetic of this kind at all, not because the lock
catches a divergence that was about to corrupt a hash.

## Threats to validity

- One probe, one set of Math inputs. A wider input sweep could still surface the
  predicted transcendental divergence; this experiment did not attempt one,
  because transcendentals are outside what a plan evaluates.
- `qjs` is the standalone interpreter, not the embedded `rquickjs` the tool will
  use. They share the QuickJS-ng core, so engine-level number and string
  behaviour is the same, but the embedding has not itself been run cross-platform.
- Darwin coverage is aarch64 only; no x86_64-darwin was tested, and none is in
  the fleet.
