import test from "node:test";
import assert from "node:assert/strict";

const BASE_URL = "http://localhost:7070";

test("node status reports a valid running LOKYX node", async () => {
  const response = await fetch(`${BASE_URL}/status`);

  assert.equal(response.status, 200);

  const status = await response.json();

  assert.equal(status.name, "LOKYX");
  assert.equal(status.port, 7070);
  assert.ok(Number.isInteger(status.height));
  assert.ok(status.height >= 0);
  assert.match(status.latestHash, /^[a-f0-9]{64}$/);
});

test("chain endpoint returns a chain beginning at genesis", async () => {
  const response = await fetch(`${BASE_URL}/chain`);

  assert.equal(response.status, 200);

  const chain = await response.json();

  assert.ok(Array.isArray(chain));
  assert.ok(chain.length >= 1);
  assert.equal(chain[0].index, 0);
  assert.equal(chain[0].previousHash, "0".repeat(64));
});
