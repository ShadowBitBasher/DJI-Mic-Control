import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { normalizeLocale, translate, zhCN } from "./translations.js";

test("English is the default and unknown locales do not opt into Chinese", () => {
  for (const value of [null, undefined, "", "fr", "zh-TW"]) {
    assert.equal(normalizeLocale(value), "en");
  }
  assert.equal(normalizeLocale("zh-CN"), "zh-CN");
  for (const key of Object.keys(zhCN)) assert.equal(translate("en", key), key);
});

test("missing translations fall back without hiding new device labels", () => {
  assert.equal(translate("zh-CN", "Future device setting"), "Future device setting");
});

test("interpolation preserves device text and does not interpret replacement syntax", () => {
  assert.equal(translate("zh-CN", 'Turn on "{setting}" first', { setting: "降噪" }), "请先开启“降噪”");
  assert.equal(translate("en", "Battery: {label}", { label: "$& {literal}" }), "Battery: $& {literal}");
  assert.equal(translate("zh-CN", "Battery: {label}"), "电量：{label}");
});

test("all current model labels, groups, and warnings have translations", () => {
  const model = readFileSync(new URL("../../../crates/protocol/src/models/mic_mini.rs", import.meta.url), "utf8");
  const strings = [...model.matchAll(/(?:label:\s*|const G_\w+: &str =\s*|note: Some\()"([^"]+)"/g)].map((m) => m[1]);
  assert.ok(strings.length > 20);
  for (const label of strings) assert.ok(Object.hasOwn(zhCN, label), label);
});
