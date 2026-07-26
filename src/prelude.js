// The `compass` prelude (decision 0011, 0013).
//
// A plan module does `import { plan, step, reconcile, evidence } from "compass"`.
// This library provides construction only: nothing here reads a clock, a file,
// the network, or the environment, so a plan authored with it is pure by
// construction rather than by policy.
//
// The evidence *vocabulary* (test, review, measurement, waiver, …) is NOT
// shipped: it belongs to the use case. `evidence` is a thin forwarder that turns
// `evidence.anything(attrs)` into `atom("anything", attrs)`, and exposes the
// combinators `all` / `any` / `not`. Compass fixes the *structure* of a
// criterion and never its vocabulary (decision 0013).

function atom(kind, attrs) {
  return { __k: "atom", kind: String(kind), attrs: attrs || {} };
}
function all() {
  return { __k: "all", args: Array.prototype.slice.call(arguments) };
}
function any() {
  return { __k: "any", args: Array.prototype.slice.call(arguments) };
}
function not(p) {
  return { __k: "not", arg: p };
}

// `evidence.test({...})` === `atom("test", {...})`. No vocabulary is declared;
// the domain word is just a string handed to `atom`.
const evidence = new Proxy(
  { atom: atom, all: all, any: any, not: not },
  {
    get: function (target, prop) {
      if (Object.prototype.hasOwnProperty.call(target, prop)) return target[prop];
      if (typeof prop !== "string") return undefined;
      return function (attrs) {
        return atom(prop, attrs);
      };
    },
  },
);

function makeStep(fields) {
  const s = {
    __k: "step",
    __name: undefined,
    __origin: undefined,
    work: fields.work,
    dependsOn: fields.dependsOn || [],
    accept: fields.accept,
    supersedes: fields.supersedes,
    retired: false,
  };
  s.with = function (patch) {
    patch = patch || {};
    const n = makeStep({
      work: "work" in patch ? patch.work : s.work,
      dependsOn: "dependsOn" in patch ? patch.dependsOn : s.dependsOn,
      accept: "accept" in patch ? patch.accept : s.accept,
      supersedes: "supersedes" in patch ? patch.supersedes : s.supersedes,
    });
    // Identity is the binding; `.with` never changes it. A carried-forward
    // step keeps the identity it was born with, and records where it came from.
    n.__name = s.__name;
    n.__origin = s.__origin || s;
    n.retired = s.retired;
    return n;
  };
  return s;
}

function step(fields) {
  return makeStep(fields);
}

// Attach a step's declared name. The host appends one call per exported binding
// after the module body, so by the time a successor imports this module every
// declared step already knows the name it was declared under. Never overwrites
// a name a step was already born with (a carried-forward step keeps its origin).
globalThis.__compass_register = function (val, name) {
  if (val && val.__k === "step" && val.__name === undefined) {
    val.__name = name;
  }
};

function stepsMap(arr) {
  const m = {};
  for (let i = 0; i < arr.length; i++) {
    const s = arr[i];
    if (s && s.__k === "step" && s.__name !== undefined) m[s.__name] = s;
  }
  return m;
}

function cloneCarry(s) {
  const n = makeStep({
    work: s.work,
    dependsOn: s.dependsOn,
    accept: s.accept,
    supersedes: s.supersedes,
  });
  n.__name = s.__name;
  n.__origin = s.__origin || s;
  n.retired = s.retired;
  return n;
}

function makePlan(spec) {
  const steps = (spec.steps || []).slice();
  const p = {
    __k: "plan",
    author: spec.author,
    why: spec.why,
    goal: spec.goal,
    retired: spec.retired || false,
    __steps: steps,
  };
  Object.defineProperty(p, "steps", {
    get: function () {
      return stepsMap(p.__steps);
    },
  });
  p.revise = function (rev) {
    return reviseOne(p, rev);
  };
  return p;
}

function plan(spec) {
  return makePlan(spec);
}

// Apply the edit / add / retire triad against a carried-forward step list.
// There is no fourth operation: a step of a predecessor is carried forward
// unless it is edited or retired, so dropping a step has no spelling.
function applyOps(steps, rev) {
  const edit = rev.edit || [];
  for (let i = 0; i < edit.length; i++) {
    const e = edit[i];
    const idx = findByName(steps, e.__name);
    if (idx < 0) throw new Error("edit targets a step that is not present: " + String(e.__name));
    steps[idx] = e;
  }
  const retire = rev.retire || [];
  for (let i = 0; i < retire.length; i++) {
    const r = retire[i];
    const idx = findByName(steps, r.__name);
    if (idx < 0) throw new Error("retire targets a step that is not present: " + String(r.__name));
    steps[idx].retired = true;
  }
  const add = rev.add || [];
  for (let i = 0; i < add.length; i++) {
    steps.push(add[i]);
  }
  return steps;
}

function findByName(steps, name) {
  for (let i = 0; i < steps.length; i++) {
    if (steps[i].__name === name) return i;
  }
  return -1;
}

function reviseOne(base, rev) {
  let steps = base.__steps.map(cloneCarry);
  steps = applyOps(steps, rev);
  return makePlan({
    author: rev.author,
    why: rev.why,
    goal: "goal" in rev ? rev.goal : base.goal,
    retired: "retired" in rev ? rev.retired : base.retired,
    steps: steps,
  });
}

// A reconciliation is a revision with more than one predecessor. Every step of
// every side is carried forward, keyed by identity, so nothing is lost by
// choosing a side; the version states only what it changes.
function reconcile(rev) {
  const sides = rev.revises || [];
  const seen = {};
  const carried = [];
  for (let i = 0; i < sides.length; i++) {
    const ss = sides[i].__steps || [];
    for (let j = 0; j < ss.length; j++) {
      const s = ss[j];
      if (s.__name === undefined) continue;
      if (!Object.prototype.hasOwnProperty.call(seen, s.__name)) {
        seen[s.__name] = true;
        carried.push(cloneCarry(s));
      }
    }
  }
  const steps = applyOps(carried, rev);
  const goal = "goal" in rev ? rev.goal : sides.length > 0 ? sides[0].goal : undefined;
  return makePlan({
    author: rev.author,
    why: rev.why,
    goal: goal,
    retired: rev.retired || false,
    steps: steps,
  });
}

export { plan, step, reconcile, evidence, atom, all, any, not };
