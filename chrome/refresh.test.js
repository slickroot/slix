import assert from "node:assert/strict";
import test from "node:test";

import { REFRESH_MAX_MS, REFRESH_MIN_MS, nextRefreshDelayMs, shouldReload } from "./refresh.js";

test("nextRefreshDelayMs", async (t) => {
  await t.test("is the minimum delay when random is 0", () => {
    assert.equal(nextRefreshDelayMs(0), REFRESH_MIN_MS);
  });

  await t.test("is the midpoint when random is 0.5", () => {
    assert.equal(nextRefreshDelayMs(0.5), (REFRESH_MIN_MS + REFRESH_MAX_MS) / 2);
  });

  await t.test("stays below the maximum delay when random is close to 1", () => {
    assert.ok(nextRefreshDelayMs(0.999999) < REFRESH_MAX_MS);
  });
});

test("shouldReload", async (t) => {
  await t.test("is true for the home timeline without focus", () => {
    assert.equal(shouldReload("https://x.com/home", false), true);
  });

  await t.test("is false for the home timeline with focus", () => {
    assert.equal(shouldReload("https://x.com/home", true), false);
  });

  for (const url of ["https://x.com/someone/status/123", "https://x.com/explore"]) {
    await t.test(`is false for ${url} without focus`, () => {
      assert.equal(shouldReload(url, false), false);
    });

    await t.test(`is false for ${url} with focus`, () => {
      assert.equal(shouldReload(url, true), false);
    });
  }
});
