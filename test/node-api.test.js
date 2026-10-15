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

test("node rejects a malformed transaction without changing chain height", async () => {
  const beforeResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(beforeResponse.status, 200);
  const before = await beforeResponse.json();

  const response = await fetch(`${BASE_URL}/transactions/receive`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tx: { type: "transfer" } }),
  });

  assert.equal(response.status, 400);

  const result = await response.json();
  assert.equal(typeof result.error, "string");

  const afterResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(afterResponse.status, 200);
  const after = await afterResponse.json();

  assert.equal(after.height, before.height);
  assert.equal(after.latestHash, before.latestHash);
});

test("node rejects a transaction with an invalid signature without changing pending count", async () => {
  const beforeResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(beforeResponse.status, 200);
  const before = await beforeResponse.json();

  const response = await fetch(`${BASE_URL}/transactions/receive`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tx: {
        type: "transfer",
        sender: "LKX0000000000000000000000000000000000000000",
        recipient: "LKX1111111111111111111111111111111111111111",
        amount: 1,
        nonce: 1,
        publicKey: "not-a-valid-public-key",
        signature: "invalid-signature",
      },
    }),
  });

  assert.equal(response.status, 400);

  const result = await response.json();
  assert.equal(typeof result.error, "string");

  const afterResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(afterResponse.status, 200);
  const after = await afterResponse.json();

  assert.equal(after.pendingTransactions, before.pendingTransactions);
  assert.equal(after.height, before.height);
  assert.equal(after.latestHash, before.latestHash);
});


test("node rejects an invalid block without changing the chain", async () => {
  const beforeResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(beforeResponse.status, 200);
  const before = await beforeResponse.json();

  const response = await fetch(`${BASE_URL}/receive-block`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      block: {
        index: before.height,
        timestamp: Date.now(),
        transactions: [],
        previousHash: before.latestHash,
        nonce: 0,
        hash: "invalid-hash",
      },
    }),
  });

  assert.equal(response.status, 400);

  const result = await response.json();
  assert.equal(result.accepted, false);
  assert.equal(typeof result.message, "string");

  const afterResponse = await fetch(`${BASE_URL}/status`);
  assert.equal(afterResponse.status, 200);
  const after = await afterResponse.json();

  assert.equal(after.height, before.height);
  assert.equal(after.latestHash, before.latestHash);
});
