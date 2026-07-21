# The JavaScript runtime is embedded, not invoked

Status: accepted

## Context

Plans are authored as pure TypeScript and must be evaluated to produce a stored
version. Evaluation can either shell out to an external runtime or run in
process against an embedded engine.

Two properties make this more than a packaging preference. Identity is a hash of
the evaluated result, so **evaluation must be deterministic** or the same source
yields different versions. And a plan is a program, so **evaluation must not
grant the program any capability** — a plan has no business reading a file,
opening a socket, or asking the time.

## Evidence and Argument

Both requirements point the same way, and the second decides it.

An external runtime hands the program a complete platform. Purity can only be
*policed* — globals stubbed before the program loads, a boundary that is a speed
bump rather than a wall, since anything the runtime provides may be reachable by
a path the stubbing did not anticipate. An embedded engine starts with nothing
and receives only what the host names. That is a difference in kind: the
capability is absent rather than removed.

Measurement made the packaging side one-sided too. Two independent prototypes
built the full pipeline. The embedded path costs roughly three and a half
megabytes of binary and half a minute of build, links only the C library, and
adds **no marginal closure at all**. An external runtime costs an order of
magnitude more, delivered as a separate artifact that must be present, found,
and version-matched at runtime.

A full-featured engine was ruled out on a sharper ground than size: its build
requires network access on every path — either downloading a prebuilt archive or
downloading its own toolchain. A hermetic build must therefore pin fetched
artifacts by hash, which turns every engine upgrade into a fixed-output repair.
That cost recurs forever and is paid in the most tedious currency available.

A pure-native-language engine avoids a C dependency but costs more than twice
the binary for a less proven implementation. The C dependency is a small, fast,
network-free build; that is not the constraint worth optimizing.

Determinism was measured rather than assumed: repeated evaluation of one input
produced a single distinct result across hundreds of runs, with a negative
control confirming the check detects change, and results were byte-identical
across three engine versions spanning a wide range.

## Options

| Option | Tradeoffs |
| --- | --- |
| Embed a small interpreter | No external dependency, no marginal closure, capabilities absent by construction; adds a C build dependency and a 0.x transformer |
| Invoke an external runtime | Familiar and no engine to embed; an order of magnitude more closure, a process boundary, and purity can only be policed |
| Embed a full optimizing engine | Best-proven determinism for floating point; network-dependent builds that make every upgrade a fixed-output repair, for an order of magnitude more size |
| Embed a pure-native-language engine | No C dependency at all; more than twice the binary, less proven, for a constraint that was not binding |

## Decision

The engine is embedded in the binary. Compass invokes no external runtime and
requires none to be installed.

Evaluation happens against a global object the host **constructs explicitly**.
Nothing is available unless it was deliberately added. This is an allowlist
rather than a denylist, because a denylist widens silently whenever the engine
gains a global.

Excluded, and each for a stated reason: the clock and anything derived from it;
randomness; locale-aware behaviour, whose data is versioned independently of the
engine and changes underneath a stable engine version; anything that makes
garbage collection observable; and the transcendental floating-point functions,
which the engine delegates to the platform library and which therefore disagree
between platforms in the final bits.

Compilation and evaluation use **separate contexts**. Compiling a module
requires an evaluation capability that the sandbox must not retain, so a module
is compiled in a disposable context and the compiled form is loaded into a
locked one. Dynamic code construction consequently exists but is inert.

**Identity is the hash of the evaluated result, never of transpiled source.** The
transformer is young and changes often; hashing its output would make every
upgrade an identity migration. Hashing the evaluated value absorbs that entirely,
so long as semantics are preserved.

## Consequences

- Compass remains a single binary with no runtime prerequisite.
- A plan cannot read a file, open a socket, or observe the time, because those
  capabilities are never introduced rather than being taken away.
- One exception is honest: the randomness primitive arrives with the engine's
  base objects and cannot be structurally omitted. It is removed explicitly, and
  it is the only capability whose absence depends on the host remembering.
- Excluding the transcendental functions is what makes the same source produce
  the same version on every platform. Without it, identity is platform-local.
- The transformer is a young dependency that changes frequently. Hashing
  evaluated values rather than emitted source is what makes that acceptable, and
  it must not be quietly reversed for convenience.
- Determinism is a property to be tested, not assumed. It rests on an explicit
  allowlist, and any addition to that list is a change to this decision.
