import assert from "node:assert/strict";
import test from "node:test";

import { TAKEOFF_MAX_AGE_MS, TAKEOFF_VIEWS_PER_MIN, isTakingOff, parseViews } from "./takeoff.js";

const NOW = Date.parse("2026-01-01T12:00:00Z");

test("parseViews", async (t) => {
  await t.test("reads the view count out of a full action bar label", () => {
    assert.equal(
      parseViews("48 replies, 5 reposts, 271 likes, 17 bookmarks, 30107 views"),
      30107,
    );
  });

  await t.test("reads a single view", () => {
    assert.equal(parseViews("1 view"), 1);
  });

  await t.test("strips commas from the view count", () => {
    assert.equal(parseViews("12,345 views"), 12345);
  });

  await t.test("is null when the label has no views part", () => {
    assert.equal(parseViews("48 replies, 5 reposts, 271 likes, 17 bookmarks"), null);
  });

  await t.test("is null for an empty label", () => {
    assert.equal(parseViews(""), null);
  });

  await t.test("is null for a missing label", () => {
    assert.equal(parseViews(undefined), null);
  });
});

test("isTakingOff", async (t) => {
  const at = (ms) => new Date(NOW - ms).toISOString();

  await t.test("is true at the views per minute boundary", () => {
    assert.equal(isTakingOff(at(40 * 60000), 4000, NOW), true);
  });

  await t.test("is false just below the views per minute boundary", () => {
    assert.equal(isTakingOff(at(40 * 60000), 3999, NOW), false);
  });

  await t.test("is true just under the max age with plenty of views", () => {
    assert.equal(isTakingOff(at(TAKEOFF_MAX_AGE_MS - 1000), 1000000, NOW), true);
  });

  await t.test("is false at exactly the max age however many views", () => {
    assert.equal(isTakingOff(at(TAKEOFF_MAX_AGE_MS), 1000000, NOW), false);
  });

  await t.test("is false beyond the max age however many views", () => {
    assert.equal(isTakingOff(at(TAKEOFF_MAX_AGE_MS + 60000), 1000000, NOW), false);
  });

  await t.test("is false seconds old below the views per minute floor", () => {
    assert.equal(isTakingOff(at(10000), TAKEOFF_VIEWS_PER_MIN - 1, NOW), false);
  });

  await t.test("is true seconds old at the views per minute floor", () => {
    assert.equal(isTakingOff(at(10000), TAKEOFF_VIEWS_PER_MIN, NOW), true);
  });

  await t.test("is true for a future datetime at the floor", () => {
    assert.equal(isTakingOff(new Date(NOW + 60000).toISOString(), TAKEOFF_VIEWS_PER_MIN, NOW), true);
  });

  await t.test("is false for a future datetime below the floor", () => {
    assert.equal(
      isTakingOff(new Date(NOW + 60000).toISOString(), TAKEOFF_VIEWS_PER_MIN - 1, NOW),
      false,
    );
  });

  await t.test("is false when views are hidden", () => {
    assert.equal(isTakingOff(at(60000), null, NOW), false);
  });

  await t.test("is false for an unparseable datetime", () => {
    assert.equal(isTakingOff("not a date", 100000, NOW), false);
  });

  await t.test("is false for an empty datetime", () => {
    assert.equal(isTakingOff("", 100000, NOW), false);
  });

  await t.test("is false for a missing datetime", () => {
    assert.equal(isTakingOff(undefined, 100000, NOW), false);
  });
});
