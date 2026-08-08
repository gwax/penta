import assert from "node:assert/strict";
import test from "node:test";

import {
  chooseWorktreePort,
  devPortOverrideVariable,
  getWorktreeDevPort,
  parseDevPortOverride,
  parseWorktreeRoots,
  primaryWorktreePort,
} from "../worktree-port.js";

const roots = [
  "/projects/penta",
  "/projects/worktrees/bravo/penta",
  "/projects/worktrees/alpha/penta",
];

test("the primary checkout keeps port 3000 and linked choices are distinct", () => {
  const claimedPorts = new Set([primaryWorktreePort]);
  const firstPort = chooseWorktreePort(roots[1], claimedPorts);
  claimedPorts.add(firstPort);
  const secondPort = chooseWorktreePort(roots[2], claimedPorts);

  assert.equal(primaryWorktreePort, 3000);
  assert.notEqual(firstPort, secondPort);
  assert.ok(firstPort >= 10_000 && firstPort <= 49_151);
  assert.ok(secondPort >= 10_000 && secondPort <= 49_151);
});

test("a persisted assignment does not move when a colliding worktree is added", () => {
  const collisionOptions = {
    portStart: 4100,
    portEnd: 4101,
    hash: () => 0,
  };
  const persistedAssignments = new Map();
  const existingPort = chooseWorktreePort(roots[1], new Set(), collisionOptions);
  persistedAssignments.set(roots[1], existingPort);

  const newPort = chooseWorktreePort(
    roots[2],
    new Set(persistedAssignments.values()),
    collisionOptions,
  );

  assert.equal(persistedAssignments.get(roots[1]), 4100);
  assert.equal(newPort, 4101);
});

test("NUL-delimited porcelain output preserves unusual worktree paths", () => {
  const unusualRoot = "/projects/worktrees/line\nbreak/penta";
  const porcelain = [
    `worktree ${roots[0]}`,
    "HEAD abc123",
    "branch refs/heads/main",
    "",
    `worktree ${unusualRoot}`,
    "HEAD def456",
    "detached",
    "",
    "",
  ].join("\0");

  assert.deepEqual(parseWorktreeRoots(porcelain), [roots[0], unusualRoot]);
});

test("an environment override pins the port for every caller", () => {
  // Editor and agent previews pin one port in a static config file. Both the
  // server and `dev:url` read the override, so they cannot disagree.
  const env = { [devPortOverrideVariable]: "3000" };
  assert.equal(getWorktreeDevPort({ env }), 3000);
  assert.equal(getWorktreeDevPort({ env: { [devPortOverrideVariable]: " 8080 " } }), 8080);
});

test("an absent or blank override falls back to the worktree assignment", () => {
  assert.equal(parseDevPortOverride(undefined), undefined);
  assert.equal(parseDevPortOverride(""), undefined);
  assert.equal(parseDevPortOverride("   "), undefined);
});

test("a malformed override fails loudly instead of silently picking a port", () => {
  for (const value of ["zero", "0", "-1", "65536", "3000.5"]) {
    assert.throws(
      () => parseDevPortOverride(value),
      new RegExp(`${devPortOverrideVariable} must be a port number`),
      `expected ${value} to be rejected`,
    );
  }
});
