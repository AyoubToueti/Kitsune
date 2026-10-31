#!/usr/bin/env node
// Keep Cargo.lock in step with the workspace version.
//
// Usage:
//   node scripts/sync-version.mjs            # propagate the workspace version
//   node scripts/sync-version.mjs 0.2.0      # set the workspace version to 0.2.0
//
// The single source of truth is `[workspace.package] version` in the ROOT
// Cargo.toml. Every crate inherits it with `version.workspace = true`, so
// Cargo is the authority: it validates the value and regenerates the lockfile
// on the next build.
//
// That leaves this script with one job. Cargo.lock records each workspace
// member's own version, and it is only rewritten when something forces a
// resolve -- so bumping the version and committing would otherwise leave two
// stale lines in the lock, which the next `cargo build` would then silently
// dirty. Rewriting them here keeps the working tree clean.
//
// This used to be a four-file dance driven by `src-tauri/tauri.conf.json`.
// That file, and the Tauri app that owned it, are gone.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const WORKSPACE_TOML = join(root, "Cargo.toml");
const LOCK = join(root, "Cargo.lock");

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

/** Every workspace member whose version Cargo.lock records. */
const MEMBERS = ["kitsune-core", "kitsune-qt"];

/**
 * Read `version` from the `[workspace.package]` table.
 *
 * Deliberately not a YAML/TOML parser: the repo has no dependency on one, and
 * the table is three lines. The section boundary is respected so a
 * `version =` inside `[profile.release]` cannot be picked up by mistake.
 */
function readWorkspaceVersion(source) {
  const section = source.split(/^\[/m).find((chunk) => chunk.startsWith("workspace.package]"));
  if (!section) throw new Error("Cargo.toml has no [workspace.package] table");
  const match = section.match(/^version\s*=\s*"([^"]*)"/m);
  if (!match) throw new Error("[workspace.package] has no version");
  return match[1];
}

/** Replace the version inside `[workspace.package]`, leaving everything else. */
function setWorkspaceVersion(source, version) {
  const re = /(\[workspace\.package\][^[]*?^version\s*=\s*")([^"]*)(")/ms;
  if (!re.test(source)) throw new Error('no version line in [workspace.package]');
  return source.replace(re, `$1${version}$3`);
}

/** Replace the version of one named `[[package]]` entry in Cargo.lock. */
function setLockVersion(source, name, version) {
  const re = new RegExp(
    `(\\[\\[package\\]\\]\\nname = "${name}"\\nversion = ")([^"]*)(")`,
  );
  if (!re.test(source)) throw new Error(`no [[package]] entry for "${name}"`);
  return source.replace(re, `$1${version}$3`);
}

/** Read a named `[[package]]` version from Cargo.lock, for the report. */
function lockVersion(source, name) {
  const re = new RegExp(
    `\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]*)"`,
  );
  return source.match(re)?.[1];
}

function main() {
  const arg = process.argv[2];

  const workspaceSource = readFileSync(WORKSPACE_TOML, "utf8");
  const before = readWorkspaceVersion(workspaceSource);

  let version;
  if (arg) {
    if (!SEMVER.test(arg)) {
      console.error(`error: "${arg}" is not a valid semver version`);
      process.exit(1);
    }
    version = arg;
  } else {
    version = before;
  }

  // One read of the lock, all edits applied to it, one write. Writing per
  // member would re-read the file and clobber the earlier replacement.
  const lockSource = readFileSync(LOCK, "utf8");
  const lockBefore = Object.fromEntries(
    MEMBERS.map((name) => [name, lockVersion(lockSource, name)]),
  );

  if (version !== before) {
    writeFileSync(WORKSPACE_TOML, setWorkspaceVersion(workspaceSource, version));
  }

  let updatedLock = lockSource;
  for (const name of MEMBERS) {
    updatedLock = setLockVersion(updatedLock, name, version);
  }
  if (updatedLock !== lockSource) {
    writeFileSync(LOCK, updatedLock);
  }

  const moved = [
    ...(version !== before ? [[`workspace`, before]] : []),
    ...MEMBERS.filter((name) => lockBefore[name] !== version).map((name) => [
      `Cargo.lock:${name}`,
      lockBefore[name],
    ]),
  ];

  if (moved.length === 0) {
    console.log(`all files already at ${version}`);
  } else {
    for (const [name, from] of moved) {
      console.log(`${name}: ${from} -> ${version}`);
    }
    console.log(`\nversion set to ${version}`);
  }
}

main();