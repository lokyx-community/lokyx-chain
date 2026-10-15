
import http from "node:http";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT || 7070);
const NETWORK_ID = process.env.LOKYX_NETWORK_ID || "lokyx-testnet-1";
const NETWORK_NAME = "LOKYX Testnet";
const DATA_DIR = path.join(__dirname, `data-${PORT}`);
const CHAIN_FILE = path.join(DATA_DIR, "chain.json");
const PEERS_FILE = path.join(DATA_DIR, "peers.json");
const DIFFICULTY = Number(process.env.DIFFICULTY || 3);
const REWARD = 50;
const MAX_BODY = 1_000_000;

fs.mkdirSync(DATA_DIR, { recursive: true });

const sha256 = (s) => crypto.createHash("sha256").update(s).digest("hex");
const stable = (x) => JSON.stringify(x);
const addressFromPublicKey = (pem) => "LKX" + sha256(pem).slice(0, 40);

function blockHash(block) {
  return sha256(stable({
    index: block.index,
    timestamp: block.timestamp,
    transactions: block.transactions,
    previousHash: block.previousHash,
    nonce: block.nonce
  }));
}

function makeGenesis() {
  const b = {
    index: 0,
    timestamp: 1735689600000,
    transactions: [],
    previousHash: "0".repeat(64),
    nonce: 0
  };

  b.hash = blockHash(b);
  return b;
}

function loadJson(file, fallback) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return fallback;
  }
}

let chain = loadJson(CHAIN_FILE, [makeGenesis()]);
let peers = loadJson(PEERS_FILE, []);
let mempool = [];

function save() {
  fs.writeFileSync(CHAIN_FILE, JSON.stringify(chain, null, 2));
  fs.writeFileSync(PEERS_FILE, JSON.stringify(peers, null, 2));
}

function verifyTransaction(tx) {
  if (!tx || typeof tx !== "object" || tx.type !== "transfer") {
    return false;
  }

  if (!Number.isSafeInteger(tx.amount) || tx.amount <= 0) {
    return false;
  }

  if (
    typeof tx.sender !== "string" ||
    typeof tx.recipient !== "string"
  ) {
    return false;
  }

  if (tx.sender !== addressFromPublicKey(tx.publicKey)) {
    return false;
  }

  try {
    return crypto.verify(
      null,
      Buffer.from(stable({
        sender: tx.sender,
        recipient: tx.recipient,
        amount: tx.amount,
        nonce: tx.nonce
      })),
      tx.publicKey,
      Buffer.from(tx.signature, "base64")
    );
  } catch {
    return false;
  }
}

function balancesFor(candidateChain) {
  const balances = {};

  for (const b of candidateChain) {
    for (const tx of b.transactions) {
      if (tx.type === "reward") {
        balances[tx.recipient] =
          (balances[tx.recipient] || 0) + tx.amount;
      } else if (tx.type === "transfer") {
        balances[tx.sender] =
          (balances[tx.sender] || 0) - tx.amount;

        balances[tx.recipient] =
          (balances[tx.recipient] || 0) + tx.amount;
      }
    }
  }

  return balances;
}

function validateChain(candidate) {
  if (!Array.isArray(candidate) || candidate.length < 1) {
    return false;
  }

  const g = makeGenesis();

  if (stable(candidate[0]) !== stable(g)) {
    return false;
  }

  for (let i = 1; i < candidate.length; i++) {
    const b = candidate[i];
    const prev = candidate[i - 1];

    if (!b || typeof b !== "object") {
      return false;
    }

    if (
      b.index !== i ||
      b.previousHash !== prev.hash ||
      b.hash !== blockHash(b)
    ) {
      return false;
    }

    if (!b.hash.startsWith("0".repeat(DIFFICULTY))) {
      return false;
    }

    if (!Array.isArray(b.transactions) || b.transactions.length < 1) {
      return false;
    }

    const rewards = b.transactions.filter(t => t?.type === "reward");

    if (
      rewards.length !== 1 ||
      b.transactions[0]?.type !== "reward" ||
      rewards[0].amount !== REWARD ||
      typeof rewards[0].recipient !== "string" ||
      !rewards[0].recipient.trim()
    ) {
      return false;
    }

    for (const tx of b.transactions.filter(t => t?.type !== "reward")) {
      if (!verifyTransaction(tx)) {
        return false;
      }
    }
  }

  // Validate spend balances and transaction nonces in chain order.
  const balances = {};
  const nonces = {};

  for (const b of candidate.slice(1)) {
    for (const tx of b.transactions) {
      if (tx.type === "reward") {
        balances[tx.recipient] =
          (balances[tx.recipient] || 0) + tx.amount;
      } else {
        const expected = (nonces[tx.sender] || 0) + 1;

        if (tx.nonce !== expected) {
          return false;
        }

        if ((balances[tx.sender] || 0) < tx.amount) {
          return false;
        }

        balances[tx.sender] -= tx.amount;

        balances[tx.recipient] =
          (balances[tx.recipient] || 0) + tx.amount;

        nonces[tx.sender] = expected;
      }
    }
  }

  return true;
}

function nextNonce(address) {
  let n = 0;

  for (const b of chain) {
    for (const tx of b.transactions) {
      if (tx.type === "transfer" && tx.sender === address) {
        n = Math.max(n, tx.nonce);
      }
    }
  }

  for (const tx of mempool) {
    if (tx.sender === address) {
      n = Math.max(n, tx.nonce);
    }
  }

  return n + 1;
}

function projectedBalances() {
  const b = balancesFor(chain);

  for (const tx of mempool) {
    b[tx.sender] = (b[tx.sender] || 0) - tx.amount;
    b[tx.recipient] = (b[tx.recipient] || 0) + tx.amount;
  }

  return b;
}

function addTransaction(tx) {
  if (!verifyTransaction(tx)) {
    throw new Error("Invalid transaction signature or fields.");
  }

  if (
    mempool.some(
      t => t.sender === tx.sender && t.nonce === tx.nonce
    )
  ) {
    throw new Error("Duplicate sender nonce.");
  }

  if (tx.nonce !== nextNonce(tx.sender)) {
    throw new Error(`Wrong nonce. Expected ${nextNonce(tx.sender)}.`);
  }

  if ((projectedBalances()[tx.sender] || 0) < tx.amount) {
    throw new Error("Insufficient balance.");
  }

  mempool.push(tx);
}

function mineBlock(miner) {
  if (
    !miner ||
    typeof miner !== "string" ||
    miner.length > 128
  ) {
    throw new Error("Provide a miner address.");
  }

  const txs = [...mempool];
  const rewardTx = {
    type: "reward",
    recipient: miner,
    amount: REWARD
  };

  const b = {
    index: chain.length,
    timestamp: Date.now(),
    transactions: [rewardTx, ...txs],
    previousHash: chain.at(-1).hash,
    nonce: 0
  };

  const target = "0".repeat(DIFFICULTY);

  do {
    b.hash = blockHash(b);

    if (b.hash.startsWith(target)) {
      break;
    }

    b.nonce++;
  } while (true);

  chain.push(b);
  mempool = [];
  save();

  return b;
}

function json(res, status, obj) {
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "content-type",
    "access-control-allow-methods": "GET,POST,OPTIONS"
  });

  res.end(JSON.stringify(obj, null, 2));
}

async function readBody(req) {
  let data = "";

  for await (const chunk of req) {
    data += chunk;

    if (data.length > MAX_BODY) {
      throw new Error("Request too large.");
    }
  }

  return data ? JSON.parse(data) : {};
}

async function requestJson(url, method = "GET", body) {
  const response = await fetch(url, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(2500)
  });

  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }

  return response.json();
}

async function broadcastBlock(block) {
  await Promise.allSettled(
    peers.map(async peer => {
      await requestJson(
        `${peer}/receive-block`,
        "POST",
        { block }
      );
    })
  );
}

const server = http.createServer(async (req, res) => {
  if (req.method === "OPTIONS") {
    return json(res, 204, {});
  }

  const url = new URL(req.url, `http://localhost:${PORT}`);

  try {
    if (req.method === "GET" && url.pathname === "/") {
      return json(res, 200, {
        name: "LOKYX",
        version: "0.1.0-prototype",
        port: PORT,
        warning:
          "Experimental local prototype. Not production-secure; no real-value tokens.",
        endpoints: [
          "/status",
          "/chain",
          "/balance?address=LKX...",
          "/wallet/new",
          "/mine?address=LKX...",
          "/peers",
          "/sync"
        ]
      });
    }

    if (req.method === "GET" && url.pathname === "/status") {
      return json(res, 200, {
        name: "LOKYX",
        networkName: NETWORK_NAME,
        networkId: NETWORK_ID,
        port: PORT,
        height: chain.length - 1,
        difficulty: DIFFICULTY,
        pendingTransactions: mempool.length,
        peers,
        latestHash: chain.at(-1).hash
      });
    }

    if (req.method === "GET" && url.pathname === "/chain") {
      return json(res, 200, chain);
    }

    if (req.method === "GET" && url.pathname === "/balance") {
      const address = url.searchParams.get("address") || "";

      return json(res, 200, {
        address,
        balance: balancesFor(chain)[address] || 0,
        unit: "LOKYX (test only)"
      });
    }

    if (req.method === "GET" && url.pathname === "/wallet/new") {
      const { publicKey, privateKey } =
        crypto.generateKeyPairSync("ed25519");

      const publicPem = publicKey.export({
        type: "spki",
        format: "pem"
      }).toString();

      const privatePem = privateKey.export({
        type: "pkcs8",
        format: "pem"
      }).toString();

      return json(res, 200, {
        warning:
          "DEMO ONLY: save the private key securely; this endpoint is unauthenticated and must never be exposed publicly.",
        address: addressFromPublicKey(publicPem),
        publicKey: publicPem,
        privateKey: privatePem
      });
    }

    if (req.method === "POST" && url.pathname === "/transactions") {
      const body = await readBody(req);
      const privateKey = crypto.createPrivateKey(body.privateKey);
      const publicKey = crypto.createPublicKey(privateKey);

      const publicPem = publicKey.export({
        type: "spki",
        format: "pem"
      }).toString();

      const sender = addressFromPublicKey(publicPem);

      const txBody = {
        sender,
        recipient: body.recipient,
        amount: Number(body.amount),
        nonce: Number(body.nonce) || nextNonce(sender)
      };

      if (
        !Number.isSafeInteger(txBody.amount) ||
        txBody.amount <= 0
      ) {
        throw new Error("Amount must be a positive integer.");
      }

      const signature = crypto.sign(
        null,
        Buffer.from(stable(txBody)),
        privateKey
      ).toString("base64");

      const tx = {
        type: "transfer",
        ...txBody,
        publicKey: publicPem,
        signature
      };

      addTransaction(tx);

      await Promise.allSettled(
        peers.map(peer =>
          requestJson(
            `${peer}/transactions/receive`,
            "POST",
            { tx }
          )
        )
      );

      return json(res, 201, { accepted: true, tx });
    }

    if (
      req.method === "POST" &&
      url.pathname === "/transactions/receive"
    ) {
      const { tx } = await readBody(req);
      addTransaction(tx);

      return json(res, 201, { accepted: true });
    }

    if (req.method === "GET" && url.pathname === "/mine") {
      const address = url.searchParams.get("address");

      if (!address) {
        throw new Error("Use /mine?address=YOUR_LKX_ADDRESS");
      }

      const block = mineBlock(address);
      await broadcastBlock(block);

      return json(res, 200, { mined: true, block });
    }

    if (req.method === "POST" && url.pathname === "/peers") {
      const { peer } = await readBody(req);
      const normalized = String(peer || "").replace(/\/+$/, "");
      const u = new URL(normalized);

      if (
        !["http:", "https:"].includes(u.protocol) ||
        u.hostname === "0.0.0.0"
      ) {
        throw new Error("Provide a valid reachable peer URL.");
      }

      if (
        normalized === `http://localhost:${PORT}` ||
        normalized === `http://127.0.0.1:${PORT}`
      ) {
        throw new Error("Cannot add this node as its own peer.");
      }

      await requestJson(`${normalized}/status`);

      if (!peers.includes(normalized)) {
        peers.push(normalized);
      }

      save();

      return json(res, 201, { peers });
    }

    if (req.method === "GET" && url.pathname === "/peers") {
      return json(res, 200, peers);
    }

    if (req.method === "GET" && url.pathname === "/sync") {
      const results = await Promise.allSettled(
        peers.map(p => requestJson(`${p}/chain`))
      );

      let adopted = false;

      for (const r of results) {
        if (
          r.status === "fulfilled" &&
          r.value.length > chain.length &&
          validateChain(r.value)
        ) {
          chain = r.value;
          mempool = [];
          save();
          adopted = true;
        }
      }

      return json(res, 200, {
        adopted,
        height: chain.length - 1,
        valid: validateChain(chain)
      });
    }

    if (
      req.method === "POST" &&
      url.pathname === "/receive-block"
    ) {
      const { block } = await readBody(req);

      // Reject malformed submissions before attempting chain validation.
      if (
        !block ||
        typeof block !== "object" ||
        !Number.isInteger(block.index)
      ) {
        return json(res, 400, {
          accepted: false,
          message: "Invalid block."
        });
      }

      // A block at an existing height is a duplicate or stale submission.
      if (block.index <= chain.length - 1) {
        return json(res, 409, {
          accepted: false,
          message:
            "Block height is already present; request sync if needed."
        });
      }

      // A future block that skips the next expected height is out of sequence.
      if (block.index > chain.length) {
        return json(res, 409, {
          accepted: false,
          message:
            "Block height does not match the next expected height."
        });
      }

      // The block is at the expected height. Validate it before accepting it.
      const candidate = [...chain, block];

      if (!validateChain(candidate)) {
        return json(res, 400, {
          accepted: false,
          message: "Invalid block."
        });
      }

      chain = candidate;

      mempool = mempool.filter(tx =>
        !block.transactions.some(
          t => t.signature && t.signature === tx.signature
        )
      );

      save();

      return json(res, 200, { accepted: true });
    }

    return json(res, 404, { error: "Not found" });
  } catch (e) {
    return json(res, 400, {
      error: e.message || "Bad request"
    });
  }
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(
    `LOKYX prototype node listening on http://localhost:${PORT}`
  );

  console.log(`Data directory: ${DATA_DIR}`);

  console.log(
    "Prototype only. Do not expose to the public internet or use real funds."
  );
});
