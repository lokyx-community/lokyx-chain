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
        index: before.height + 1,
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


test("node rejects a correctly signed transaction with a negative nonce", async () => {
  const { publicKey, privateKey } = await import("node:crypto").then(({ generateKeyPairSync }) =>
    generateKeyPairSync("ed25519")
  );

  const crypto = await import("node:crypto");
  const publicPem = publicKey.export({ type: "spki", format: "pem" }).toString();
  const privatePem = privateKey.export({ type: "pkcs8", format: "pem" }).toString();
  const { createHash, sign } = crypto;

  const sender = "LKX" + createHash("sha256").update(publicPem).digest("hex").slice(0, 40);
  const txBody = {
    sender,
    recipient: "LKX1111111111111111111111111111111111111111",
    amount: 1,
    nonce: -1,
  };

  const signature = sign(
    null,
    Buffer.from(JSON.stringify(txBody)),
    privatePem
  ).toString("base64");

  const before = await (await fetch(`${BASE_URL}/status`)).json();

  const response = await fetch(`${BASE_URL}/transactions/receive`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tx: { type: "transfer", ...txBody, publicKey: publicPem, signature },
    }),
  });

  assert.equal(response.status, 400);

  const after = await (await fetch(`${BASE_URL}/status`)).json();
  assert.equal(after.pendingTransactions, before.pendingTransactions);
  assert.equal(after.height, before.height);
  assert.equal(after.latestHash, before.latestHash);
});


test("node rejects a block with the reward transaction out of position", async () => {
  const crypto = await import("node:crypto");
  const { publicKey, privateKey } = crypto.generateKeyPairSync("ed25519");

  const publicPem = publicKey.export({
    type: "spki",
    format: "pem"
  }).toString();

  const privatePem = privateKey.export({
    type: "pkcs8",
    format: "pem"
  }).toString();

  const sender = "LKX" + crypto.createHash("sha256")
    .update(publicPem)
    .digest("hex")
    .slice(0, 40);

  const txBody = {
    sender,
    recipient: "LKX1111111111111111111111111111111111111111",
    amount: 1,
    nonce: 1
  };

  const tx = {
    type: "transfer",
    ...txBody,
    publicKey: publicPem,
    signature: crypto.sign(
      null,
      Buffer.from(JSON.stringify(txBody)),
      privatePem
    ).toString("base64")
  };

  const before = await (await fetch(`${BASE_URL}/status`)).json();
  const chain = await (await fetch(`${BASE_URL}/chain`)).json();
  const previous = chain.at(-1);

  const block = {
    index: previous.index + 1,
    timestamp: Date.now(),
    transactions: [
      tx,
      {
        type: "reward",
        recipient: "LKXTEST000000000000000000000000000000000001",
        amount: 50
      }
    ],
    previousHash: previous.hash,
    nonce: 0
  };

  const hashBlock = value => crypto.createHash("sha256")
    .update(JSON.stringify({
      index: value.index,
      timestamp: value.timestamp,
      transactions: value.transactions,
      previousHash: value.previousHash,
      nonce: value.nonce
    }))
    .digest("hex");

  do {
    block.hash = hashBlock(block);
    if (block.hash.startsWith("000")) break;
    block.nonce++;
  } while (true);

  const response = await fetch(`${BASE_URL}/receive-block`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ block })
  });

  assert.equal(response.status, 400);

  const after = await (await fetch(`${BASE_URL}/status`)).json();
  assert.equal(after.height, before.height);
  assert.equal(after.latestHash, before.latestHash);
});
