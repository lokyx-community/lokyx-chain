# LOKYX

LOKYX is an experimental blockchain I'm building with Node.js. The idea is to develop a native chain and understand the parts that make it work, starting with block creation, transaction validation, and communication between nodes.

Right now, the project runs locally and supports proof-of-work mining, signed transfers, and basic synchronization between nodes. It is still under development.

## Current implementation

- Blocks are hashed using SHA-256.
- Mining uses a proof-of-work check with a configurable difficulty.
- Transactions use Ed25519 signatures.
- Transfers include balance checks and sequential nonces.
- Nodes can connect to peers and exchange blocks.
- The chain is saved locally so the node can load its state after restarting.
- A basic HTML block explorer is included.

## Running locally

You'll need Node.js 20 or newer.

Clone the repository, open its directory, and run:

```bash
npm start
```

By default, the node listens on port `7070`.

To start a second local node in PowerShell, open another terminal and run:

```powershell
$env:PORT = "7071"
npm start
```

The nodes use separate local data directories. Keep both terminals open while testing.

## Useful endpoints

| Endpoint | Purpose |
|---|---|
| `/status` | View the node's current status |
| `/chain` | Inspect the local blockchain |
| `/balance?address=LKX...` | Check an address balance |
| `/wallet/new` | Generate a test key pair |
| `/mine?address=LKX...` | Mine a block |
| `/peers` | View configured peers |
| `/sync` | Try to synchronize with a peer |

Some endpoints change the node's state. Use this API only in a local test environment.

## Current status

I've tested the basic two-node setup locally. Both nodes reported the same chain height and block hash after a block was mined on one node and received by the other.

That confirms the basic block-propagation path works in this test. It doesn't mean the consensus mechanism or validation rules are ready for production.

## What's left to do

- Add automated tests for valid and invalid transactions.
- Test malformed blocks and incorrect mining rewards.
- Improve fork handling and chain synchronization.
- Review peer communication and API security.
- Make wallet key handling safer.
- Expand the explorer and document the protocol.

## Important

LOKYX is experimental software. Don't expose these nodes to the public internet, store valuable keys in them, or use them to handle real funds. The current wallet and API design need further security work before public deployment.

Contributions, issues, and technical feedback are welcome.
