# LOKYX — one-hour blockchain prototype

A runnable Node.js prototype demonstrating:
- SHA-256 proof-of-work blocks
- Native demo-unit rewards (`LOKYX`)
- Ed25519-signed transfer transactions
- Balance checks and sequential nonces
- Multiple local nodes that exchange transactions and blocks
- Longer-valid-chain synchronization

## Requirements
- Node.js 20+
- No npm dependencies

## Run two nodes (Windows PowerShell)

Open two terminal windows in this folder.

Terminal 1:
```powershell
$env:PORT=7070
node lokyx.js
```

Terminal 2:
```powershell
$env:PORT=7071
node lokyx.js
```

In a third PowerShell window, connect the nodes:
```powershell
Invoke-RestMethod -Method Post -Uri http://localhost:7070/peers -ContentType 'application/json' -Body '{"peer":"http://localhost:7071"}'
Invoke-RestMethod -Method Post -Uri http://localhost:7071/peers -ContentType 'application/json' -Body '{"peer":"http://localhost:7070"}'
```

## Create a demo wallet

Open:
`http://localhost:7070/wallet/new`

Save the returned `address` and `privateKey` somewhere safe for this local demo. This endpoint is intentionally simple and is NOT safe to expose publicly. Do not reuse any real wallet key.

## Mine

Replace `YOUR_ADDRESS` with the address returned above:
`http://localhost:7070/mine?address=YOUR_ADDRESS`

Check balance:
`http://localhost:7070/balance?address=YOUR_ADDRESS`

Mine on either node. If the nodes don't immediately agree, visit:
`http://localhost:7071/sync`

## Send a transaction

Use a local API client such as curl. Substitute the actual private key, recipient address, and amount. Keep the private key in your local terminal; do not paste it into a public service.

Example PowerShell (replace placeholders):
```powershell
$body = @{
  privateKey = @'
-----BEGIN PRIVATE KEY-----
PASTE_LOCAL_DEMO_PRIVATE_KEY_HERE
-----END PRIVATE KEY-----
'@
  recipient = 'LKX_RECIPIENT_ADDRESS'
  amount = 5
} | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://localhost:7070/transactions -ContentType 'application/json' -Body $body
```

Transactions require sufficient mined balance. Mine another block to include pending transactions. This prototype's API accepts private keys in request bodies, so use it only on localhost.

## Important limitations

This is a learning prototype, not a production blockchain:
- No BFT/Proof-of-Stake finality, no slashing, no peer discovery protocol, no offline Bluetooth/Wi-Fi mesh.
- Proof of work is intentionally easy for demos and can be changed with `DIFFICULTY`.
- No production-grade mempool policy, fee market, reorg handling, wallet encryption, or rate limiting.
- Chain validation is minimal and needs more adversarial tests before public use.
- Demo rewards and coin units have no monetary value.
- Do not port this directly to mainnet or expose its unauthenticated HTTP API to the internet.
