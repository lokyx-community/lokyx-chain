# LOKYX — Independent Layer 1 Blockchain

**An independent Layer 1 blockchain in development, focused on native digital assets, peer-to-peer networking, protocol validation, and a custom execution environment.**

<p align="center">

**Protocol Research · Node Development · Testnet Engineering**

</p>

LOKYX is an experimental blockchain project exploring the design and implementation of an independent Layer 1 network with its own native asset, node software, transaction model, and consensus protocol.

The project follows an iterative engineering approach: establish protocol behavior, validate blockchain invariants through automated tests, and progressively develop a native node implementation for multi-node testing.

> **Development status:** Experimental prototype. The native Rust node, finalized consensus protocol, and public testnet are still under development.

---

## Table of Contents

- [Vision](#vision)
- [Core Design Goals](#core-design-goals)
- [Architecture](#architecture)
- [Protocol Prototyping](#protocol-prototyping)
- [Current Prototype](#current-prototype)
- [Technology Stack](#technology-stack)
- [Getting Started](#getting-started)
- [Running Tests](#running-tests)
- [Running Local Nodes](#running-local-nodes)
- [HTTP API](#http-api)
- [Security Considerations](#security-considerations)
- [Development Roadmap](#development-roadmap)
- [Contributing](#contributing)
- [Disclaimer](#disclaimer)

---

## Vision

The goal of LOKYX is to build an independent blockchain network rather than simply deploy a token or application on an existing blockchain.

The long-term vision includes:

- A native Layer 1 protocol and native asset.
- Independently operated blockchain nodes.
- Peer-to-peer communication and chain synchronization.
- Cryptographically signed transactions.
- A formally specified consensus mechanism.
- A native wallet and address format.
- A custom virtual machine for future programmable execution.
- Support for local peer communication and global network connectivity.
- A developer-friendly ecosystem with a blockchain explorer and public APIs.

These are development objectives, not claims that every component is currently implemented.

## Core Design Goals

### 1. Protocol independence

LOKYX is intended to define its own network rules, genesis configuration, transaction format, block structure, and node implementation.

### 2. Verifiable state transitions

Every node should independently validate transactions and blocks instead of trusting data received from another peer.

### 3. Native asset

LOKYX is designed around its own native coin, intended to support transaction fees and protocol-level economic functions. Issuance, supply limits, and fee rules must be formally specified before a public testnet launch.

### 4. Peer-to-peer networking

Nodes should discover peers, exchange transactions and blocks, synchronize chain state, and recover reliably after network interruptions.

### 5. Consensus research

A hybrid consensus approach is being explored. Its final rules, security assumptions, validator selection, Sybil resistance, and fault tolerance must be specified and tested before it can be considered production-ready.

### 6. Extensible execution

The initial priority is the native blockchain protocol. A custom virtual machine is a planned direction for future programmable execution.

---

## Architecture

LOKYX is being developed in stages.

```text
                 LOKYX Protocol
                       |
          +------------+------------+
          |            |            |
      Transactions   Blocks      Consensus
          |            |            |
          +------------+------------+
                       |
              Validation Engine
                       |
                Chain State
                       |
              Peer-to-Peer Layer
                       |
             Independent LOKYX Nodes
                       |
          +------------+------------+
          |            |            |
        Wallet       Explorer    Developer APIs
```

This diagram describes the intended architecture. The current JavaScript prototype implements only part of this design.

### Protocol components

| Component | Purpose |
|---|---|
| Block structure | Organizes transactions into linked blocks |
| Cryptographic hashing | Identifies blocks and links them to previous blocks |
| Transaction validation | Checks transaction structure and validity |
| Digital signatures | Verifies transaction authorization |
| Nonce validation | Helps prevent transaction replay |
| Chain validation | Checks block linkage and protocol rules |
| Node API | Exposes blockchain operations over HTTP |
| Peer synchronization | Exchanges chain data between nodes |
| Native execution environment | Planned custom execution capability |

---

## Protocol Prototyping

The initial protocol prototype was implemented in JavaScript using Node.js to establish and test the core blockchain rules before moving toward a native node implementation.

This phase focuses on making blockchain behavior reproducible and testable.

The prototype provides a foundation for evaluating:

- Block creation and hash linkage.
- Proof-of-work validation.
- Transaction signatures and nonce handling.
- Chain integrity checks.
- Invalid transaction rejection.
- Invalid block rejection.
- Node API behavior.
- Initial peer synchronization workflows.

The JavaScript implementation serves as a protocol reference during development. The planned Rust implementation will need to reproduce the same formally defined protocol behavior rather than merely duplicate the existing code.

### Automated validation

The current test suite has **7 passing tests** covering:

1. Node status and basic health information.
2. Chain retrieval and genesis validation.
3. Rejection of malformed transactions.
4. Rejection of transactions with invalid signatures.
5. Rejection of invalid blocks.
6. Rejection of correctly signed transactions with negative nonces.
7. Rejection of blocks with incorrectly positioned reward transactions.

These results describe the current local test suite. They do not establish production security, consensus safety, or public-network readiness.

---

## Current Prototype

The repository currently contains a JavaScript/Node.js blockchain prototype with local chain storage, HTTP endpoints, and automated API tests.

### Implemented in the prototype

- Basic block and transaction structures.
- Hash-linked chain validation.
- Proof-of-work checks.
- Transaction signature verification.
- Nonce and balance-related validation.
- Reward transaction validation.
- HTTP-based node endpoints.
- Initial peer registration and synchronization routes.
- Automated tests for selected valid and invalid cases.

### Under active development

- Network identity enforcement across peer communication.
- Robust chain synchronization and peer validation.
- Startup validation of persisted blockchain state.
- Secure transaction signing and private-key management.
- Formal consensus specification.
- Native Rust node implementation.
- Independent multi-node network testing.

### Not yet claimed as complete

- A production-secure consensus protocol.
- A deployed public testnet.
- A finalized native coin economic model.
- A completed custom virtual machine.
- Production-grade wallet security.
- Audited protocol cryptography.

---

## Technology Stack

### Current prototype

| Technology | Role |
|---|---|
| JavaScript | Prototype implementation |
| Node.js | Runtime for the prototype |
| HTTP and JSON | Node API and initial peer communication |
| Git | Version control |
| Node.js test runner | Automated validation |

### Planned native implementation

| Technology | Intended role |
|---|---|
| Rust | Native blockchain node and protocol implementation |
| Cryptographic libraries | Established primitives for signatures and hashing |
| Persistent storage | Reliable chain and state storage |
| Peer-to-peer networking | Communication between independent nodes |
| Automated integration tests | Cross-node validation and synchronization tests |

Specific libraries and storage technologies will be selected as the protocol design is finalized.

---

## Getting Started

### Prerequisites

For the current JavaScript prototype:

- Node.js
- npm
- Git

For native node development:

- Rust toolchain
- Cargo

### Clone the repository

```bash
git clone https://github.com/lokyx-community/lokyx-chain.git
cd lokyx-chain
```

### Install dependencies

```bash
npm install
```

### Run the prototype

```bash
npm start
```

The default node port and local data-directory behavior are defined in the current implementation. Consult the source code and project configuration before starting additional nodes.

### Run the test suite

```bash
npm test
```

The repository's current test command uses the Node.js built-in test runner.

---

## Running Local Nodes

The prototype has been developed with separate local node data directories, including `data-7070` and `data-7071`.

These directories represent local development state; they are not evidence of a deployed public network.

For repeatable multi-node testing:

1. Start nodes using distinct ports and separate data directories.
2. Verify that each node reports its own status and chain height.
3. Register peers using the implemented API.
4. Check that peers belong to the same configured network.
5. Submit valid and invalid transactions.
6. Test block propagation and chain synchronization.
7. Restart nodes and verify that persisted state is loaded and validated correctly.
8. Confirm that nodes converge on the same valid chain under the specified protocol rules.

Do not assume that running two local HTTP servers establishes a secure decentralized network. Peer authentication, network isolation, consensus, recovery, and adversarial behavior require dedicated testing.

---

## HTTP API

The current prototype exposes HTTP endpoints for node information, chain access, wallet operations, balances, mining, and peer synchronization.

| Endpoint | Method | Purpose |
|---|---|---|
| `/` | GET | Node information and endpoint overview |
| `/status` | GET | Node status, configured network identity, height, and peer information |
| `/chain` | GET | Retrieve the chain |
| `/balance` | GET | Query an address balance |
| `/wallet/new` | GET | Create a wallet using the current prototype implementation |
| `/mine` | GET | Trigger the prototype's mining operation |
| `/peers` | GET | List configured peers |
| `/peers` | POST | Register a peer |
| `/sync` | GET | Trigger chain synchronization |
| `/receive-block` | POST | Receive a propagated block |
| `/transactions/receive` | POST | Receive a propagated transaction |

**Important:** Endpoint behavior and required request fields are implementation-specific and may change during development. Inspect `lokyx.js` before integrating a client.

The current prototype's wallet and mining endpoints are not suitable for an untrusted public deployment. In particular, private keys must not be exposed through public node APIs, and state-changing operations should use appropriate authenticated methods.

---

## Security Considerations

Security is a protocol requirement, not a feature that can be inferred from successful tests.

The following areas require further design, implementation, and verification before a public testnet or production deployment.

### Consensus security

- Define the consensus algorithm and its assumptions.
- Specify validator or miner eligibility.
- Address Sybil attacks and malicious participants.
- Define fork choice and conflict resolution.
- Test network partitions and competing chains.

### Cryptography

- Use established, reviewed cryptographic libraries.
- Specify canonical transaction and block serialization.
- Verify signatures and transaction authorization.
- Define key generation, storage, recovery, and rotation.
- Avoid custom cryptographic primitives.

### Networking

- Validate every message received from peers.
- Enforce network identity and protocol-version compatibility.
- Apply request-size limits, timeouts, and rate limits.
- Handle malicious peers and synchronization failures.
- Avoid treating peer-provided chain data as trusted.

### Storage and recovery

- Validate persisted chain state during startup.
- Handle corrupted or incomplete writes safely.
- Preserve consistent state during crashes and restarts.
- Test recovery from invalid local data.

### Deployment

- Use a dedicated testnet configuration and genesis definition.
- Separate testnet data from development data.
- Protect administrative interfaces and secrets.
- Monitor node health and network behavior.
- Publish clear instructions for joining and operating a node.

The prototype is experimental and should not be used to secure real-value assets.

---

## Development Roadmap

### Phase 1 — Protocol prototyping

- [x] Implement initial block and transaction structures.
- [x] Implement basic chain validation.
- [x] Add transaction signature checks.
- [x] Add invalid transaction and block tests.
- [x] Establish a local JavaScript prototype.
- [x] Achieve seven passing tests in the current test suite.

### Phase 2 — Protocol specification and native node

- [ ] Specify canonical transaction and block formats.
- [ ] Define deterministic genesis and network parameters.
- [ ] Formalize the consensus protocol.
- [ ] Implement the native Rust node.
- [ ] Reproduce protocol validation with automated tests.
- [ ] Verify compatibility between implementations where applicable.

### Phase 3 — Networking and state integrity

- [ ] Enforce network identity across all peer messages.
- [ ] Harden peer registration and synchronization.
- [ ] Validate persisted state at startup.
- [ ] Improve transaction propagation and block recovery.
- [ ] Add integration tests for multiple independent nodes.
- [ ] Test network interruptions and malicious input.

### Phase 4 — Testnet preparation

- [ ] Finalize initial testnet genesis parameters.
- [ ] Deploy independently operated, reachable nodes.
- [ ] Establish peer discovery and operational monitoring.
- [ ] Publish node installation and configuration instructions.
- [ ] Test restart, synchronization, and recovery scenarios.
- [ ] Document known limitations and security assumptions.

### Phase 5 — Ecosystem development

- [ ] Develop the native wallet and address format.
- [ ] Build a blockchain explorer.
- [ ] Finalize the native asset and fee model.
- [ ] Design and implement the custom virtual machine.
- [ ] Expand developer tooling and APIs.
- [ ] Commission independent security reviews before production use.

Roadmap items will be marked complete as the corresponding implementation and verification work is finished.

---

## Contributing

Contributions, code reviews, testing, and protocol discussions are welcome.

Useful areas for contribution include:

- Blockchain protocol design.
- Rust systems programming.
- Cryptographic engineering.
- Peer-to-peer networking.
- Consensus research.
- Automated testing.
- Developer tools and blockchain explorers.
- Documentation and reproducible deployment.

### Suggested contribution workflow

1. Review the existing implementation and open issues.
2. Discuss protocol changes before introducing incompatible behavior.
3. Write tests for new behavior and relevant failure cases.
4. Run the existing test suite.
5. Document changes to protocol rules, APIs, and configuration.
6. Submit a pull request with a concise technical explanation.

Protocol changes should prioritize determinism, security, backward compatibility, and testability.

---

## Project Status

LOKYX is an experimental independent Layer 1 blockchain project progressing from protocol prototyping toward a native node implementation and multi-node testnet.

The current JavaScript prototype establishes an initial testable foundation. The next major engineering milestones are formal protocol specification, native implementation, stronger networking and state validation, and repeatable multi-node testing.

**The objective is to build a verifiable blockchain protocol and a network that independent operators can run—not simply a demonstration of blocks on a local machine.**

---

## Disclaimer

LOKYX is provided for research and development purposes. The software and protocol are experimental, may contain vulnerabilities, and have not been established as production-ready.

No guarantee is made regarding security, availability, transaction finality, asset value, or fitness for any particular purpose. Do not use the prototype to hold real-value assets or expose unprotected development endpoints to the public internet.

Consult the repository's license for the applicable terms of use and contribution.
