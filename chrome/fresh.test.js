import assert from "node:assert/strict";
import test from "node:test";

import { FRESH_MS, isFresh, isHomeTimeline } from "./fresh.js";

const NOW = Date.parse("2026-01-01T12:00:00Z");

test("isFresh", async (t) => {
  await t.test("is fresh when just published", () => {
    assert.equal(isFresh("2026-01-01T12:00:00Z", NOW), true);
  });

  await t.test("is fresh just under the cutoff", () => {
    assert.equal(isFresh(new Date(NOW - FRESH_MS + 1000).toISOString(), NOW), true);
  });

  await t.test("is not fresh at exactly the cutoff", () => {
    assert.equal(isFresh(new Date(NOW - FRESH_MS).toISOString(), NOW), false);
  });

  await t.test("is not fresh beyond the cutoff", () => {
    assert.equal(isFresh(new Date(NOW - FRESH_MS - 60000).toISOString(), NOW), false);
  });

  await t.test("is not fresh for an unparseable datetime", () => {
    assert.equal(isFresh("not a date", NOW), false);
  });

  await t.test("is not fresh for an empty datetime", () => {
    assert.equal(isFresh("", NOW), false);
  });

  await t.test("is not fresh for a missing datetime", () => {
    assert.equal(isFresh(undefined, NOW), false);
  });

  await t.test("is fresh for a future datetime", () => {
    assert.equal(isFresh(new Date(NOW + 60000).toISOString(), NOW), true);
  });
});

test("isHomeTimeline", async (t) => {
  await t.test("is true on the home timeline", () => {
    assert.equal(isHomeTimeline("https://x.com/home"), true);
  });

  await t.test("is true on the home timeline with a query string", () => {
    assert.equal(isHomeTimeline("https://x.com/home?x=1"), true);
  });

  await t.test("is true on twitter.com", () => {
    assert.equal(isHomeTimeline("https://twitter.com/home"), true);
  });

  await t.test("is false below the home path", () => {
    assert.equal(isHomeTimeline("https://x.com/home/foo"), false);
  });

  await t.test("is false at the root", () => {
    assert.equal(isHomeTimeline("https://x.com/"), false);
  });

  await t.test("is false on a profile", () => {
    assert.equal(isHomeTimeline("https://x.com/someuser"), false);
  });

  await t.test("is false on a status", () => {
    assert.equal(isHomeTimeline("https://x.com/someuser/status/123"), false);
  });

  await t.test("is false on explore", () => {
    assert.equal(isHomeTimeline("https://x.com/explore"), false);
  });

  await t.test("is false on notifications", () => {
    assert.equal(isHomeTimeline("https://x.com/notifications"), false);
  });

  await t.test("is false on search", () => {
    assert.equal(isHomeTimeline("https://x.com/search?q=a"), false);
  });
});