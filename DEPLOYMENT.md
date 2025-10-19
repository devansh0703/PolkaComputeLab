# PolkaComputeLab Deployment Guide

## Prerequisites

### System Requirements
- **CPU**: 4+ cores recommended
- **RAM**: 8GB minimum, 16GB recommended  
- **Storage**: 100GB+ SSD
- **OS**: Ubuntu 20.04/22.04, macOS, or Windows (WSL2)

### Software Requirements
```bash
# Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup update
rustup target add wasm32-unknown-unknown

# Build dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y git clang curl libssl-dev llvm libudev-dev protobuf-compiler

# Build dependencies (macOS)
brew install openssl cmake llvm protobuf
```

## Local Development

### 1. Clone and Build

```bash
# Clone repository
git clone https://github.com/polkacomputelab/polkacomputelab.git
cd polkacomputelab

# Build the project
./build.sh

# Or manually:
cargo build --release
```

### 2. Run Tests

```bash
# Run all tests
./test.sh

# Or individually:
cargo test --all
cargo test -p pallet-job-registry
cargo test -p pallet-job-verifier
```

### 3. Run Local Development Node

```bash
# Start node in development mode
./target/release/polkacomputelab-node --dev

# With detailed logging
RUST_LOG=debug ./target/release/polkacomputelab-node --dev --tmp

# Purge chain data and restart fresh
./target/release/polkacomputelab-node purge-chain --dev
./target/release/polkacomputelab-node --dev
```

### 4. Interact with Node

**Using Polkadot.js Apps:**
1. Go to https://polkadot.js.org/apps/
2. Connect to `ws://127.0.0.1:9944`
3. Navigate to Developer → Extrinsics
4. Submit jobs, verify proofs, switch consensus, etc.

**Using RPC:**
```bash
# Get chain info
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' http://localhost:9944

# Get latest block
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' http://localhost:9944
```

## Local Parachain Setup

### 1. Setup Relay Chain

```bash
# Clone Polkadot
git clone https://github.com/paritytech/polkadot-sdk.git
cd polkadot-sdk
cargo build --release --bin polkadot

# Start Alice (validator 1)
./target/release/polkadot \
  --alice \
  --validator \
  --base-path /tmp/relay/alice \
  --chain rococo-local \
  --port 30333 \
  --rpc-port 9944 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001

# In another terminal, start Bob (validator 2)
./target/release/polkadot \
  --bob \
  --validator \
  --base-path /tmp/relay/bob \
  --chain rococo-local \
  --port 30334 \
  --rpc-port 9945 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

### 2. Setup Parachain

```bash
# Export genesis state and wasm
cd polkacomputelab
./target/release/polkacomputelab-node export-genesis-state --chain local > genesis-state
./target/release/polkacomputelab-node export-genesis-wasm --chain local > genesis-wasm

# Start collator
./target/release/polkacomputelab-node \
  --alice \
  --collator \
  --force-authoring \
  --chain local \
  --base-path /tmp/parachain/alice \
  --port 40333 \
  --rpc-port 8844 \
  -- \
  --execution wasm \
  --chain rococo-local \
  --port 30343 \
  --rpc-port 9977
```

### 3. Register Parachain

**Via Polkadot.js Apps UI:**
1. Connect to relay chain: `ws://localhost:9944`
2. Go to Developer → Sudo
3. Select `paraSudoWrapper` → `sudoScheduleParaInitialize`
4. Parameters:
   - `id`: 2000 (or your chosen ParaId)
   - `genesis`: Upload `genesis-state` file
   - `validationCode`: Upload `genesis-wasm` file
   - `parachain`: true
5. Submit transaction
6. Wait for next epoch (~10 minutes in Rococo Local)

## Testnet Deployment (Westend)

### 1. Get Testnet Tokens

```bash
# Get WND tokens from faucet
# Visit: https://faucet.polkadot.io/westend
# Or use Element chat: !drip <YOUR_ADDRESS>
```

### 2. Reserve ParaId

```bash
# Connect to Westend via Polkadot.js Apps
# Go to Network → Parachains → Parathreads
# Click "Reserve ParaId"
# Submit transaction with sufficient balance
```

### 3. Build for Westend

```bash
# Build release
cargo build --release

# Generate chain spec for Westend
./target/release/polkacomputelab-node build-spec \
  --chain westend \
  --disable-default-bootnode \
  > westend-chain-spec.json

# Edit chain spec: set ParaId to your reserved ID

# Build raw spec
./target/release/polkacomputelab-node build-spec \
  --chain westend-chain-spec.json \
  --raw \
  --disable-default-bootnode \
  > westend-chain-spec-raw.json

# Export genesis
./target/release/polkacomputelab-node export-genesis-state \
  --chain westend-chain-spec-raw.json \
  > westend-genesis-state

./target/release/polkacomputelab-node export-genesis-wasm \
  --chain westend-chain-spec-raw.json \
  > westend-genesis-wasm
```

### 4. Run Collator on Westend

```bash
./target/release/polkacomputelab-node \
  --collator \
  --name "PolkaComputeLab-Collator-1" \
  --chain westend-chain-spec-raw.json \
  --base-path /var/lib/polkacomputelab \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --rpc-methods=Safe \
  --rpc-external \
  --pruning archive \
  -- \
  --chain westend \
  --port 30334 \
  --rpc-port 9945 \
  --sync warp
```

### 5. Register on Westend

1. Connect to Westend relay via Polkadot.js
2. Go to Network → Parachains → Parathreads
3. Click "Register" next to your reserved ParaId
4. Upload genesis state and wasm
5. Submit registration transaction
6. Bid for parachain slot or remain as parathread

## Production Deployment

### 1. Security Hardening

```bash
# Create dedicated user
sudo useradd -m -s /bin/bash polkacomputelab

# Set proper permissions
sudo chown -R polkacomputelab:polkacomputelab /opt/polkacomputelab
sudo chmod 755 /opt/polkacomputelab

# Firewall rules
sudo ufw allow 30333/tcp  # P2P
sudo ufw allow 9944/tcp   # RPC (restrict to VPN/internal only)
sudo ufw enable
```

### 2. Systemd Service

Create `/etc/systemd/system/polkacomputelab.service`:

```ini
[Unit]
Description=PolkaComputeLab Collator
After=network.target

[Service]
Type=simple
User=polkacomputelab
Group=polkacomputelab
WorkingDirectory=/opt/polkacomputelab
ExecStart=/opt/polkacomputelab/polkacomputelab-node \
  --collator \
  --name "PolkaComputeLab-Prod-1" \
  --chain production-spec.json \
  --base-path /var/lib/polkacomputelab \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --rpc-methods=Safe \
  --pruning archive \
  -- \
  --chain polkadot \
  --port 30334 \
  --sync warp

Restart=always
RestartSec=10
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable polkacomputelab
sudo systemctl start polkacomputelab
sudo systemctl status polkacomputelab

# View logs
sudo journalctl -u polkacomputelab -f
```

### 3. Monitoring Setup

**Prometheus Configuration** (`prometheus.yml`):
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'polkacomputelab'
    static_configs:
      - targets: ['localhost:9615']
        labels:
          instance: 'collator-1'
```

**Start Prometheus:**
```bash
docker run -d \
  --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus
```

**Grafana Setup:**
```bash
docker run -d \
  --name=grafana \
  -p 3000:3000 \
  grafana/grafana

# Access: http://localhost:3000 (admin/admin)
# Add Prometheus data source: http://prometheus:9090
# Import Substrate dashboard ID: 13759
```

### 4. Backup Strategy

```bash
# Backup chain data
tar -czf backup-$(date +%Y%m%d).tar.gz /var/lib/polkacomputelab/chains/

# Backup keys
tar -czf keys-backup-$(date +%Y%m%d).tar.gz /var/lib/polkacomputelab/chains/*/keystore/

# Automated backup script
cat > /usr/local/bin/backup-polkacomputelab.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/backup/polkacomputelab"
DATE=$(date +%Y%m%d-%H%M%S)
mkdir -p $BACKUP_DIR

# Backup chain data
tar -czf $BACKUP_DIR/chain-$DATE.tar.gz /var/lib/polkacomputelab/chains/

# Cleanup old backups (keep last 7 days)
find $BACKUP_DIR -name "chain-*.tar.gz" -mtime +7 -delete
EOF

chmod +x /usr/local/bin/backup-polkacomputelab.sh

# Add to cron (daily at 2 AM)
echo "0 2 * * * /usr/local/bin/backup-polkacomputelab.sh" | sudo crontab -
```

## Upgrades

### Runtime Upgrade

```bash
# Build new runtime
cargo build --release -p polkacomputelab-runtime

# The wasm is at:
# target/release/wbuild/polkacomputelab-runtime/polkacomputelab_runtime.compact.compressed.wasm

# Submit upgrade via Polkadot.js:
# 1. Developer → Sudo
# 2. system → setCode(code)
# 3. Upload the wasm file
# 4. Submit transaction
```

### Node Upgrade

```bash
# Build new node
cargo build --release

# Stop service
sudo systemctl stop polkacomputelab

# Backup current binary
sudo cp /opt/polkacomputelab/polkacomputelab-node /opt/polkacomputelab/polkacomputelab-node.bak

# Copy new binary
sudo cp target/release/polkacomputelab-node /opt/polkacomputelab/

# Start service
sudo systemctl start polkacomputelab
sudo systemctl status polkacomputelab
```

## Troubleshooting

### Node Won't Start
```bash
# Check logs
sudo journalctl -u polkacomputelab -n 100

# Verify binary
./target/release/polkacomputelab-node --version

# Check permissions
ls -la /var/lib/polkacomputelab

# Purge and resync
./target/release/polkacomputelab-node purge-chain --chain <spec>
```

### Not Producing Blocks
```bash
# Check if registered as collator
# Via Polkadot.js: Network → Parachains

# Verify keys are loaded
ls /var/lib/polkacomputelab/chains/*/keystore/

# Check sync status
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  http://localhost:9944
```

### High Resource Usage
```bash
# Monitor resources
htop
df -h
iostat -x 1

# Adjust pruning
--pruning 1000  # Keep last 1000 blocks

# Reduce RPC load
--rpc-methods=Safe
--rpc-max-connections=100
```

## Support

- **Documentation**: https://docs.polkacomputelab.io
- **GitHub Issues**: https://github.com/polkacomputelab/issues
- **Discord**: https://discord.gg/polkacomputelab
- **Element**: #polkacomputelab:matrix.org

---

**Happy Computing! 🚀**
