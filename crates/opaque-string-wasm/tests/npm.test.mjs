// Tests of the built npm package, run by `build-npm.sh --test`:
// OS_PKG names the opaque-string package directory, OS_TABLES_PKG the same
// module built with compiled-in NFC tables (the reference the engine's NFC
// is compared against), OS_CORPUS the shared tests/corpus.json.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { pathToFileURL } from "node:url";

async function load(env) {
  const dir = process.env[env];
  assert.ok(dir, `${env} is not set: run build-npm.sh --test`);
  const mod = await import(pathToFileURL(join(dir, "index.js")).href);
  mod.initSync({ module: readFileSync(join(dir, "index_bg.wasm")) });
  return mod;
}

const engine = await load("OS_PKG");
const tables = await load("OS_TABLES_PKG");
const corpus = JSON.parse(readFileSync(process.env.OS_CORPUS, "utf8"));

test("this engine is supported", () => {
  assert.equal(engine.engineSupported(), true);
  assert.equal(tables.engineSupported(), true);
});

test("shared corpus", () => {
  assert.equal(engine.unicodeVersion(), corpus.unicode);
  assert.equal(engine.nfcBaseline(), corpus.nfc_baseline);
  for (const c of corpus.cases) {
    for (const m of [engine, tables]) {
      assert.equal(m.prepare(c.input), c.prepare, `${c.name}: prepare`);
      const got = m.enforce(c.input);
      if ("ok" in c.enforce) {
        assert.deepEqual(got, { ok: true, value: c.enforce.ok }, c.name);
      } else if (c.enforce.reason === "empty") {
        assert.deepEqual(got, { ok: false, reason: "empty" }, c.name);
      } else {
        assert.deepEqual(
          got,
          {
            ok: false,
            reason: c.enforce.reason,
            codePoint: parseInt(c.enforce.code_point.slice(2), 16),
            index: c.enforce.index,
            count: c.enforce.count,
          },
          c.name,
        );
      }
    }
  }
});

test("unpaired surrogates are refused, not replaced", () => {
  assert.throws(() => engine.prepare("a\uD800b"), /unpaired surrogate/);
  assert.deepEqual(engine.enforce("a\uDC00"), { ok: false, reason: "lone_surrogate" });
  assert.deepEqual(engine.enforce("😀x"), { ok: true, value: "\u{1F600}x" });
});

test("vocabulary", () => {
  const names = engine.reasonNames();
  assert.equal(names.length, 8);
  assert.equal(names[0], "unassigned");
  assert.equal(names.at(-1), "unstable_normalization");
  assert.match(engine.version(), /^\d+\.\d+\.\d+$/);
  assert.equal(engine.version(), tables.version());
});

// The engine's NFC against the compiled-in tables, for every code point the
// profile accepts, alone and in the contexts where NFC reorders and
// composes. Equal results here are what make a password registered in a
// browser log in from a server-side or native client, and back.
test("engine NFC equals the compiled-in tables on every accepted code point", () => {
  const failures = [];
  for (let cp = 0; cp <= 0x10ffff; cp++) {
    if (cp >= 0xd800 && cp <= 0xdfff) continue;
    const c = String.fromCodePoint(cp);
    if (!tables.enforce(c).ok) continue;
    for (const s of [c, "a" + c, "a" + c + "́", c + "̣̀"]) {
      const want = tables.enforce(s);
      if (!want.ok) continue;
      const got = engine.enforce(s);
      if (!got.ok || got.value !== want.value) {
        failures.push(`U+${cp.toString(16).toUpperCase().padStart(4, "0")} in ${JSON.stringify(s)}`);
      }
    }
    if (failures.length > 20) break;
  }
  assert.deepEqual(failures, []);
});
