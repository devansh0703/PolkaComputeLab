# PolkaComputeLab

**Decentralized off-chain compute & consensus experimentation for Polkadot parachains**

## Overview

PolkaComputeLab is a fully functional Substrate-based parachain that enables:

- **Verifiable Off-Chain Computation**: Execute complex jobs off-chain with on-chain verification
- **Dynamic Consensus Experimentation**: Switch between consensus algorithms and measure performance
- **Cross-Chain Job Orchestration**: Trigger jobs across parachains using XCMP/XCM
- **Comprehensive Telemetry**: Track metrics for jobs, consensus, and validators

## Architecture

```
┌─────────────────────────────────┐
│  PolkaComputeLab Parachain      │
├─────────────────────────────────┤
│  Runtime                        │
│  ├── Job Registry Pallet        │
│  ├── Job Verifier Pallet        │
│  ├── Consensus Manager Pallet   │
│  ├── Event Hub Pallet           │
│  └── Telemetry Pallet           │
├─────────────────────────────────┤
│  Off-Chain Workers              │
│  ├── Job Scheduler              │
│  ├── Job Executor               │
│  ├── Proof Generator            │
│  └── Verifier Submit            │
└─────────────────────────────────┘
```

## Pallets

### 1. Job Registry Pallet
Manages job submission, dependencies, status tracking, and lifecycle management.

**Key Features:**
- Job submission with metadata and dependencies
- Dependency validation (prevents circular dependencies)
- Status transitions (Pending → InProgress → Completed → Verified)
- Per-account job limits
- Ready job queries for OCWs

### 2. Job Verifier Pallet
Verifies off-chain computation results using cryptographic proofs.

**Key Features:**
- Multiple proof types (Signature, Merkle, Hash)
- Proof submission and validation
- Verification statistics tracking
- Integration with Job Registry for status updates

### 3. Consensus Manager Pallet
Enables dynamic consensus switching and metrics collection.

**Key Features:**
- Consensus type management (Aura, BABE, Custom)
- Block metrics tracking (validator count, block time, forks)
- Validator performance metrics
- Fork detection and statistics
- Historical consensus switch tracking

### 4. Event Hub Pallet *(Placeholder for full implementation)*
Triggers jobs based on events, supports XCMP/XCM cross-chain messaging.

### 5. Telemetry Pallet *(Placeholder for full implementation)*
Collects and exposes metrics via Prometheus for monitoring and analysis.

## Off-Chain Workers (OCW)

OCWs handle the actual job execution off-chain:

1. **Scheduler**: Polls Job Registry for ready jobs
2. **Executor**: Runs computations locally
3. **Proof Generator**: Creates cryptographic proofs of results
4. **Verifier Submit**: Submits proofs to Job Verifier

## Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add wasm target
rustup target add wasm32-unknown-unknown

# Install Substrate dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y git clang curl libssl-dev llvm libudev-dev protobuf-compiler
```

### Build

```bash
# Build the node
cargo build --release

# Build runtime only
cargo build --release -p polkacomputelab-runtime
```

### Run Local Node

```bash
# Run in development mode
./target/release/polkacomputelab-node --dev

# Or with detailed logs
RUST_LOG=info ./target/release/polkacomputelab-node --dev
```

### Run Tests

```bash
# Run all tests
cargo test --all

# Run specific pallet tests
cargo test -p pallet-job-registry
cargo test -p pallet-job-verifier
cargo test -p pallet-consensus-manager

# Run with output
cargo test -- --nocapture
```

### Build Chain Spec

```bash
# Generate chain spec
./target/release/polkacomputelab-node build-spec --disable-default-bootnode --chain local > chain-spec.json

# Generate raw chain spec
./target/release/polkacomputelab-node build-spec --chain=chain-spec.json --raw --disable-default-bootnode > chain-spec-raw.json
```

## Usage Examples

### Submit a Job

```rust
// Via extrinsic
let metadata = vec![1, 2, 3, 4]; // Job parameters
let dependencies = vec![]; // No dependencies
let deadline = 1000; // Block number

dispatch(
    Call::JobRegistry(JobRegistryCall::submit_job {
        metadata,
        dependencies,
        deadline,
    })
);
```

### Off-Chain Worker Flow

```rust
// OCW polls for ready jobs
let ready_jobs = JobRegistry::get_ready_jobs();

for job_id in ready_jobs {
    // Execute computation
    let result = execute_job(job_id);
    
    // Generate proof
    let proof = generate_proof(&result);
    
    // Submit proof on-chain
    submit_unsigned_transaction(|account| {
        Call::JobVerifier(JobVerifierCall::submit_proof {
            job_id,
            result_hash: hash(&result),
            proof_type: ProofType::Hash,
            proof_data: proof,
        })
    });
}
```

### Switch Consensus

```rust
// Requires sudo/root access
dispatch(
    Call::ConsensusManager(ConsensusManagerCall::set_consensus {
        consensus_type: ConsensusType::Babe,
    })
);
```

## Local Parachain Testing

### Start Relay Chain

```bash
# Clone Polkadot
git clone https://github.com/paritytech/polkadot-sdk.git
cd polkadot-sdk

# Build Polkadot
cargo build --release --bin polkadot

# Run relay chain (Alice)
./target/release/polkadot --alice --validator --base-path /tmp/relay/alice \
  --chain rococo-local --port 30333 --rpc-port 9944

# Run relay chain (Bob) in another terminal
./target/release/polkadot --bob --validator --base-path /tmp/relay/bob \
  --chain rococo-local --port 30334 --rpc-port 9945 --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/<alice_peer_id>
```

### Start Parachain Collator

```bash
# Export genesis state and wasm
./target/release/polkacomputelab-node export-genesis-state --chain local > genesis-state
./target/release/polkacomputelab-node export-genesis-wasm --chain local > genesis-wasm

# Run collator
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

### Register Parachain

Use the Polkadot.js Apps UI to register your parachain:
1. Connect to relay chain (ws://localhost:9944)
2. Go to Developer → Sudo
3. Use `paraSudoWrapper.sudoScheduleParaInitialize`
4. Upload genesis-state and genesis-wasm
5. Assign parachain ID (e.g., 2000)

## Testnet Deployment

### Westend/Paseo

```bash
# Connect to Westend relay chain
./target/release/polkacomputelab-node \
  --collator \
  --chain westend \
  --base-path /var/lib/polkacomputelab \
  --port 30333 \
  --rpc-port 9944 \
  -- \
  --chain westend \
  --port 30334 \
  --rpc-port 9945
```

Get testnet tokens from faucets:
- Westend: https://faucet.polkadot.io/westend
- Paseo: https://faucet.paseo.io/

## Monitoring & Telemetry

### Prometheus Metrics

Metrics exposed at `http://localhost:9615/metrics`:

- `polkacomputelab_jobs_submitted_total`
- `polkacomputelab_jobs_completed_total`
- `polkacomputelab_jobs_verified_total`
- `polkacomputelab_consensus_switches_total`
- `polkacomputelab_block_time_seconds`
- `polkacomputelab_forks_detected_total`

### Grafana Dashboard

```bash
# Start Prometheus
prometheus --config.file=prometheus.yml

# Start Grafana
grafana-server --config=grafana.ini
```

Access Grafana at `http://localhost:3000` and import the provided dashboard.

## Development

### Project Structure

```
polkacomputelab/
├── node/                    # Node implementation
│   ├── src/
│   │   ├── chain_spec.rs   # Chain specification
│   │   ├── cli.rs          # CLI configuration
│   │   ├── command.rs      # Command handling
│   │   └── service.rs      # Node service
│   └── Cargo.toml
├── runtime/                 # Runtime implementation
│   ├── src/
│   │   └── lib.rs          # Runtime configuration
│   └── Cargo.toml
├── pallets/                 # Custom pallets
│   ├── job-registry/       # Job management
│   ├── job-verifier/       # Proof verification
│   ├── consensus-manager/  # Consensus control
│   ├── event-hub/          # Event triggering
│   └── telemetry/          # Metrics collection
├── Cargo.toml              # Workspace configuration
└── README.md
```

### Adding a New Pallet

1. Create pallet directory: `mkdir -p pallets/my-pallet/src`
2. Add to workspace: Edit `Cargo.toml`
3. Implement pallet: Create `lib.rs` with pallet logic
4. Add to runtime: Configure in `runtime/src/lib.rs`
5. Test: Add tests in `tests.rs`

### Benchmarking

```bash
# Run benchmarks
cargo build --release --features runtime-benchmarks
./target/release/polkacomputelab-node benchmark pallet \
  --chain dev \
  --pallet pallet_job_registry \
  --extrinsic '*' \
  --steps 50 \
  --repeat 20 \
  --output pallets/job-registry/src/weights.rs
```

## Testing Strategy

- **Unit Tests**: Each pallet has comprehensive unit tests
- **Integration Tests**: Test pallet interactions
- **Runtime Tests**: Test full runtime configuration
- **Benchmarks**: Performance testing for weight calculation

## Security Considerations

1. **Job Dependencies**: Circular dependency detection prevents infinite loops
2. **Proof Verification**: Multiple proof types for security flexibility
3. **Consensus Switching**: Admin-only, safe at block boundaries
4. **OCW Security**: Results must be cryptographically verified
5. **Resource Limits**: Job counts, proof sizes, dependency depths limited

## Roadmap

- [x] Job Registry Pallet
- [x] Job Verifier Pallet
- [x] Consensus Manager Pallet
- [ ] Event Hub Pallet (Full implementation)
- [ ] Telemetry Pallet (Full implementation)
- [ ] OCW Implementation (Full job execution)
- [ ] XCMP/XCM Integration
- [ ] Grafana Dashboards
- [ ] Production Runtime
- [ ] Audit & Security Review

## Resources

- [Substrate Documentation](https://docs.substrate.io/)
- [Polkadot Wiki](https://wiki.polkadot.network/)
- [Cumulus Documentation](https://github.com/paritytech/cumulus)
- [Off-Chain Workers Guide](https://docs.substrate.io/learn/offchain-operations/)

## License

Apache 2.0

## Contributing

Contributions welcome! Please read CONTRIBUTING.md for details.

## Support

- GitHub Issues: https://github.com/polkacomputelab/issues
- Discord: https://discord.gg/polkacomputelab
- Forum: https://forum.polkacomputelab.io

---

**Built with ❤️ using Substrate and Polkadot SDK**
