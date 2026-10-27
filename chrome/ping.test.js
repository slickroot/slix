import assert from "node:assert/strict";
import test from "node:test";

import {
  NOTIFICATION_TEXT_MAX,
  PING_MEMORY_MS,
  decidePing,
  notificationMessage,
  statusId,
} from "./ping.js";

const NOW = Date.parse("2026-01-01T12:00:00Z");

test("statusId", async (t) => {
  await t.test("reads the id out of a relative status href", () => {
    assert.equal(statusId("/jack/status/20"), "20");
  });

  await t.test("reads the id out of an absolute status url", () => {
    assert.equal(statusId("https://x.com/jack/status/20"), "20");
  });

  await t.test("is null for a href without a status", () => {
    assert.equal(statusId("/jack"), null);
  });

  await t.test("is null for an empty href", () => {
    assert.equal(statusId(""), null);
  });

  await t.test("is null for a missing href", () => {
    assert.equal(statusId(undefined), null);
  });
});

test("decidePing", async (t) => {
  await t.test("notifies a new id and remembers it at now", () => {
    assert.deepEqual(decidePing({}, "20", NOW), { notify: true, pinged: { 20: NOW } });
  });

  await t.test("does not notify a known id", () => {
    const pinged = { 20: NOW - 1000 };
    assert.deepEqual(decidePing(pinged, "20", NOW), { notify: false, pinged });
  });

  await t.test("forgets an id pinged exactly the memory ago and notifies it again", () => {
    assert.deepEqual(decidePing({ 20: NOW - PING_MEMORY_MS }, "20", NOW), {
      notify: true,
      pinged: { 20: NOW },
    });
  });

  await t.test("keeps an id pinged just under the memory ago", () => {
    const pinged = { 20: NOW - (PING_MEMORY_MS - 1) };
    assert.deepEqual(decidePing(pinged, "20", NOW), { notify: false, pinged });
  });

  await t.test("drops other stale ids from the returned map", () => {
    const pinged = { 1: NOW - PING_MEMORY_MS, 2: NOW - 1000 };
    assert.deepEqual(decidePing(pinged, "20", NOW).pinged, { 2: NOW - 1000, 20: NOW });
  });

  await t.test("does not mutate the input map", () => {
    const pinged = { 1: NOW - PING_MEMORY_MS, 2: NOW - 1000 };
    const copy = { ...pinged };
    decidePing(pinged, "20", NOW);
    assert.deepEqual(pinged, copy);
  });
});

test("notificationMessage", async (t) => {
  await t.test("returns short text as is", () => {
    assert.equal(notificationMessage("hello"), "hello");
  });

  await t.test("returns text of exactly the max length as is", () => {
    const text = "a".repeat(NOTIFICATION_TEXT_MAX);
    assert.equal(notificationMessage(text), text);
  });

  await t.test("cuts text over the max length and adds an ellipsis", () => {
    assert.equal(
      notificationMessage("a".repeat(NOTIFICATION_TEXT_MAX + 1)),
      "a".repeat(NOTIFICATION_TEXT_MAX - 1) + "…",
    );
  });

  await t.test("trims whitespace before the ellipsis", () => {
    const text = "a".repeat(NOTIFICATION_TEXT_MAX - 2) + "  " + "b".repeat(10);
    assert.equal(notificationMessage(text), "a".repeat(NOTIFICATION_TEXT_MAX - 2) + "…");
  });

  await t.test("trims surrounding whitespace", () => {
    assert.equal(notificationMessage("  hello \n"), "hello");
  });

  await t.test("is empty for empty text", () => {
    assert.equal(notificationMessage(""), "");
  });

  await t.test("is empty for missing text", () => {
    assert.equal(notificationMessage(undefined), "");
  });
});
