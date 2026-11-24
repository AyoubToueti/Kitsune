#!/usr/bin/env node
// Generate CHANGELOG.md from the git history.
//
// Usage:
//   node scripts/changelog.mjs                    # last tag..HEAD
//   node scripts/changelog.mjs HEAD --version 1.0.0
//   node scripts/changelog.mjs --version 1.1.0
//   node scripts/changelog.mjs v1.0.0..HEAD --version 1.1.0
//
// The first release covers the full history because the backend was built
// during the Tauri/SvelteKit era and only the frontend was later rewritten in
// Qt/QML. Pass `HEAD` explicitly for that case: the default range starts at the
// newest reachable tag.
//
// This replaces the former AI Changelog workflow. GitHub Models was retired on
// 2026-07-30, so its endpoint now returns a plain-text "OK" that is not JSON.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const REPO = "https://github.com/AyoubToueti/Kitsune";

const KNOWN_TYPES = new Set([
  "feat",
  "fix",
  "perf",
  "refactor",
  "docs",
  "build",
  "ci",
  "test",
  "style",
  "chore",
  "revert",
]);

const TYPE_LABELS = {
  feat: "Features",
  fix: "Fixes",
  perf: "Performance",
  refactor: "Refactoring",
  docs: "Documentation",
  build: "Build",
  ci: "CI",
  test: "Tests",
  style: "Style",
  chore: "Chores",
  revert: "Reverts",
  other: "Other changes",
};

const MAIN_TYPES = ["feat", "fix", "perf", "refactor", "other"];
const MAINTENANCE_TYPES = ["docs", "build", "ci", "test", "style", "chore", "revert"];

// Release chores and changelog-tooling commits are not user-facing changes.
const SKIP = /^(release:|chore: update CHANGELOG\.md|Add AI Changelog workflow|ci\(changelog\): allow scoping|fix\(ci\): pin the changelog)/i;

function git(args) {
  return execFileSync("git", args, { cwd: ROOT, encoding: "utf8" });
}

const argv = process.argv.slice(2);
let range = "";
let output = "CHANGELOG.md";
let version = "";

for (let i = 0; i < argv.length; i++) {
  const arg = argv[i];
  if (arg === "--output") {
    output = argv[++i];
  } else if (arg === "--version") {
    version = argv[++i];
  } else if (arg.startsWith("--")) {
    console.error(`error: unknown option "${arg}"`);
    process.exit(1);
  } else {
    range = arg;
  }
}

if (!range) {
  try {
    const tag = git(["describe", "--tags", "--abbrev=0"]).trim();
    range = `${tag}..HEAD`;
  } catch {
    range = "HEAD";
  }
}

const raw = git([
  "log",
  range,
  "--no-merges",
  "--pretty=format:%H%x1f%h%x1f%ad%x1f%s%x1e",
  "--date=short",
]);

const commits = raw
  .split("\x1e")
  .map((record) => record.trim())
  .filter(Boolean)
  .map((record) => {
    const [full, short, date, subject] = record.split("\x1f");
    const match = subject.match(
      /^([A-Za-z][A-Za-z0-9_-]*)(?:\(([^)]+)\))?(!)?:\s*(.+)$/,
    );

    let type = "other";
    let scope = "general";
    let text = subject;

    if (match) {
      const prefix = match[1].toLowerCase();
      if (KNOWN_TYPES.has(prefix)) {
        type = prefix;
        scope = match[2] || "general";
      } else {
        // A bare prefix such as `core:` or `player:` is an area, not a type.
        scope = match[1].toLowerCase();
      }
      text = match[4];
    }

    return {
      full,
      short,
      date,
      subject,
      type,
      scope,
      text,
      breaking: Boolean(match?.[3]),
    };
  })
  .filter((commit) => !SKIP.test(commit.subject));

const groups = new Map();
for (const commit of commits) {
  if (!groups.has(commit.type)) groups.set(commit.type, new Map());
  const scopes = groups.get(commit.type);
  if (!scopes.has(commit.scope)) scopes.set(commit.scope, []);
  scopes.get(commit.scope).push(commit);
}

for (const scopes of groups.values()) {
  for (const list of scopes.values()) {
    list.sort(
      (a, b) => b.date.localeCompare(a.date) || a.short.localeCompare(b.short),
    );
  }
}

function renderType(type) {
  const scopes = groups.get(type);
  if (!scopes) return "";

  const ordered = [...scopes.entries()].sort(
    (a, b) => b[1].length - a[1].length || a[0].localeCompare(b[0]),
  );

  let out = `### ${TYPE_LABELS[type] || type}\n\n`;
  for (const [scope, list] of ordered) {
    out += `**${scope}**\n\n`;
    for (const commit of list) {
      const breaking = commit.breaking ? " **BREAKING**" : "";
      out += `- ${commit.text} ([${commit.short}](${REPO}/commit/${commit.full}), ${commit.date})${breaking}\n`;
    }
    out += "\n";
  }
  return out;
}

const today = new Date().toISOString().slice(0, 10);

let changelog = "# Changelog\n\n";
changelog +=
  "All notable changes to Kitsune, generated from the git history. v1.0.0 is " +
  "the first public release, so it covers both the original Tauri/SvelteKit " +
  "era and the Qt6/QML rewrite; the backend is the same throughout.\n\n";
changelog += `## ${version || "Unreleased"} (${today})\n\n`;

for (const type of MAIN_TYPES) changelog += renderType(type);

const maintenance = MAINTENANCE_TYPES.map(renderType).join("");
if (maintenance) {
  changelog +=
    "<details>\n" +
    "<summary>Maintenance (documentation, build, CI, tests, chores)</summary>\n\n" +
    maintenance +
    "</details>\n";
}

writeFileSync(join(ROOT, output), changelog);
console.log(`wrote ${output} (${commits.length} commits, range ${range})`);
