#!/usr/bin/env node
// Keep the app version identical across the files that carry it.
//
// Usage:
//   node scripts/sync-version.mjs            # propagate tauri.conf.json's version
//   node scripts/sync-version.mjs 0.2.0      # set every file to 0.2.0
//
// `src-tauri/tauri.conf.json` is the source of truth: it is what Tauri bakes
// into the app, the installer filenames, and the release workflow's
// `tagName: v__VERSION__`. The other files are kept in lockstep so the running
// app, the npm package, and the Rust crate never disagree -- a mismatch makes
// the auto-updater compare the wrong numbers.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const FILES = {
  tauri: join(root, "src-tauri", "tauri.conf.json"),
  pkg: join(root, "package.json"),
  cargo: join(root, "src-tauri", "Cargo.toml"),
  lock: join(root, "src-tauri", "Cargo.lock"),
};

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

/** Read the version out of tauri.conf.json (the source of truth). */
function readTauriVersion() {
  const json = JSON.parse(readFileSync(FILES.tauri, "utf8"));
  if (!json.version) {
    throw new Error("tauri.conf.json has no top-level `version` field");
  }
  return json.version;
}

/**
 * Replace the first `"version": "..."` in a JSON string, preserving the rest
 * of the file byte-for-byte (so indentation and key order are untouched).
 */
function setJsonVersion(source, version) {
  const re = /("version"\s*:\s*")([^"]*)(")/;
  if (!re.test(source)) throw new Error('no "version" field found');
  return source.replace(re, `$1${version}$3`);
}

/** Replace the `version = "..."` line under [package] in Cargo.toml. */
function setCargoVersion(source, version) {
  // Anchor on the line start so we only touch the package version, never a
  // dependency's `version = "..."`.
  const re = /^version\s*=\s*"[^"]*"/m;
  if (!re.test(source)) throw new Error('no version line found');
  return source.replace(re, `version = "${version}"`);
}

/**
 * Replace the version of the `kitsune` package entry in Cargo.lock.
 *
 * The lock is a machine-written file, but it records our own crate's version
 * and `cargo build` regenerates it -- updating it here keeps the working tree
 * clean instead of leaving a stray one-line diff after every build.
 */
function setLockVersion(source, version) {
  const re = /(\[\[package\]\]\nname = "kitsune"\nversion = ")([^"]*)(")/;
  if (!re.test(source)) throw new Error('no [[package]] entry for "kitsune"');
  return source.replace(re, `$1${version}$3`);
}

/** Read the current version of each file, for the change report. */
function currentVersions() {
  return {
    tauri: readTauriVersion(),
    pkg: JSON.parse(readFileSync(FILES.pkg, "utf8")).version,
    cargo: readFileSync(FILES.cargo, "utf8").match(/^version\s*=\s*"([^"]*)"/m)?.[1],
    lock: readFileSync(FILES.lock, "utf8").match(
      /\[\[package\]\]\nname = "kitsune"\nversion = "([^"]*)"/,
    )?.[1],
  };
}

function main() {
  const arg = process.argv[2];

  let version;
  if (arg) {
    if (!SEMVER.test(arg)) {
      console.error(`error: "${arg}" is not a valid semver version`);
      process.exit(1);
    }
    version = arg;
  } else {
    version = readTauriVersion();
  }

  const before = currentVersions();

  writeFileSync(
    FILES.tauri,
    setJsonVersion(readFileSync(FILES.tauri, "utf8"), version),
  );
  writeFileSync(
    FILES.pkg,
    setJsonVersion(readFileSync(FILES.pkg, "utf8"), version),
  );
  writeFileSync(
    FILES.cargo,
    setCargoVersion(readFileSync(FILES.cargo, "utf8"), version),
  );
  writeFileSync(
    FILES.lock,
    setLockVersion(readFileSync(FILES.lock, "utf8"), version),
  );

  const changed = Object.entries(before).filter(([, v]) => v !== version);
  if (changed.length === 0) {
    console.log(`all files already at ${version}`);
  } else {
    for (const [name, v] of changed) {
      console.log(`${name}: ${v} -> ${version}`);
    }
    console.log(`\nversion set to ${version}`);
  }
}

main();