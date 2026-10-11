// Smoke tests of the built npm packages, run by `build-npm.sh --test`:
// RU_PKG and RU_FULL_PKG name the restrictive-usernames and
// restrictive-usernames-full package directories.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { pathToFileURL } from "node:url";

async function load(env, { sync = true } = {}) {
  const dir = process.env[env];
  assert.ok(dir, `${env} is not set: run build-npm.sh --test`);
  const mod = await import(pathToFileURL(join(dir, "index.js")).href);
  const bytes = readFileSync(join(dir, "index_bg.wasm"));
  if (sync) mod.initSync({ module: bytes });
  else await mod.default({ module_or_path: bytes });
  return mod;
}

const lite = await load("RU_PKG");
const full = await load("RU_FULL_PKG", { sync: false });

test("structural: rules by name", () => {
  for (const m of [lite, full]) {
    assert.ok(m.checkStructural("a.b", "username").includes("disallowed_char"));
    assert.deepEqual(m.checkStructural("ab", "username"), ["length"]);
    assert.deepEqual(m.checkStructural("", "nickname"), ["empty"]);
    assert.deepEqual(m.checkStructural("O'Brien", "nickname"), []);
  }
});

test("structural: length overrides", () => {
  assert.deepEqual(lite.checkStructural("ab", "username", 2), []);
  assert.deepEqual(lite.checkStructural("abcdef", "username", undefined, 5), ["length"]);
  assert.deepEqual(lite.checkStructural("abcdef", "username", null, 6), []);
});

test("unknown kind throws", () => {
  assert.throws(() => lite.checkStructural("abc", "Username"), /unknown kind "Username"/);
  assert.throws(() => full.normalize("abc", "user"), Error);
  assert.throws(() => lite.defaultPolicy("room"), /unknown kind/);
});

test("vocabulary", () => {
  for (const m of [lite, full]) {
    const names = m.ruleNames();
    assert.equal(names.length, 12);
    assert.equal(names[0], "empty");
    assert.equal(names.at(-1), "not_taken");
    assert.match(m.policyVersion(), /^\d+$/);
    assert.deepEqual(m.defaultPolicy("username"), {
      kind: "username",
      minLen: 3,
      maxLen: 32,
      maxMarkRun: 4,
    });
    assert.equal(m.defaultPolicy("label").maxLen, 50);
  }
  assert.equal(lite.policyVersion(), full.policyVersion());
});

test("structural package carries no Unicode policy", () => {
  for (const f of ["check", "isValid", "normalize", "keyCi", "keyCs", "unicodeVersion"]) {
    assert.equal(lite[f], undefined, f);
  }
});

test("full: check is a superset of checkStructural", () => {
  for (const [input, kind] of [
    ["a.b", "username"],
    ["Аdmin", "username"],
    ["x́́", "nickname"],
    ["  two  spaces ", "label"],
  ]) {
    const all = full.check(input, kind);
    for (const r of full.checkStructural(input, kind)) assert.ok(all.includes(r), `${input}: ${r}`);
  }
  assert.ok(full.check("Аdmin", "username").includes("mixed_script"));
  assert.equal(full.isValid("alice", "username"), true);
  assert.equal(full.isValid("al", "username"), false);
  assert.equal(full.isValid("al", "username", 2), true);
});

test("full: normalize", () => {
  assert.deepEqual(full.normalize("Ivan　Smith", "nickname"), { ok: true, value: "Ivan Smith" });
  assert.deepEqual(full.normalize(" é ", "label"), { ok: true, value: "é" });
  const r = full.normalize("Аdmin", "username");
  assert.equal(r.ok, false);
  assert.ok(r.rules.includes("mixed_script"));
  assert.ok(!r.rules.includes("not_nfc"));
});

test("full: keys", () => {
  assert.equal(full.keyCs("pаypal"), full.keyCs("paypal"));
  assert.equal(full.keyCi("pаypal"), full.keyCi("paypal"));
  assert.equal(full.keyCi("ADMIN"), full.keyCi("admin"));
  assert.notEqual(full.keyCs("ADMIN"), full.keyCs("admin"));
  assert.equal(full.keyCs("Iris"), full.keyCs("lris"));
  assert.equal(full.keyVisualCi("Dan אבי"), full.keyVisualCi("אבי Dan"));
  assert.equal(full.keyVisualCs("Dan אבי"), full.keyVisualCs("אבי Dan"));
  assert.match(full.unicodeVersion(), /^security \d+\.\d+\.\d+; /);
});
