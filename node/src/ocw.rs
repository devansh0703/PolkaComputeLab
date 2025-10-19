//! Off-Chain Worker Implementation for PolkaComputeLab
//!
//! This module provides the OCW logic for:
//! - Job scheduling and execution
//! - Proof generation and submission
//! - Event processing

use codec::{Decode, Encode};
use frame_support::traits::Get;
use frame_system::offchain::{
    AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,
};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::{
        http,
        storage::StorageValueRef,
        Duration,
    },
    traits::BlockNumberProvider,
    RuntimeDebug,
};
use sp_std::vec::Vec;

/// Key type for Off-Chain Worker
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"pcl!");

/// OCW crypto using sr25519
pub mod crypto {
    use super::KEY_TYPE;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };

    app_crypto!(sr25519, KEY_TYPE);

    pub struct OcwAuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OcwAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}

/// Job execution result
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct JobExecutionResult {
    pub job_id: u64,
    pub result_data: Vec<u8>,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// OCW Configuration trait
pub trait OffchainWorkerConfig: frame_system::Config + CreateSignedTransaction<Call<Self>> {
    /// The identifier type for an offchain worker.
    type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

    /// Call type for submitting transactions
    type RuntimeCall: From<Call<Self>>;
}

/// Offchain Worker pallet calls
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum Call<T: OffchainWorkerConfig> {
    SubmitJobResult {
        job_id: u64,
        result_hash: sp_core::H256,
        proof_data: Vec<u8>,
    },
    ProcessEvent {
        event_id: u64,
    },
    UpdateMetrics {
        job_id: u64,
        execution_time: u32,
    },
    _Phantom(sp_std::marker::PhantomData<T>),
}

/// Main OCW entry point
pub fn offchain_worker<T: OffchainWorkerConfig>(block_number: T::BlockNumber) {
    log::info!("🔧 OCW: Starting off-chain worker at block {:?}", block_number);

    // Process pending jobs
    if let Err(e) = process_pending_jobs::<T>(block_number) {
        log::error!("OCW: Error processing jobs: {:?}", e);
    }

    // Process pending events
    if let Err(e) = process_pending_events::<T>(block_number) {
        log::error!("OCW: Error processing events: {:?}", e);
    }

    // Collect metrics
    if let Err(e) = collect_metrics::<T>(block_number) {
        log::error!("OCW: Error collecting metrics: {:?}", e);
    }

    log::info!("✓ OCW: Finished off-chain worker at block {:?}", block_number);
}

/// Process pending jobs
fn process_pending_jobs<T: OffchainWorkerConfig>(
    block_number: T::BlockNumber,
) -> Result<(), &'static str> {
    log::info!("OCW: Processing pending jobs...");

    // In a real implementation, we would:
    // 1. Query the JobRegistry for ready jobs
    // 2. Execute each job
    // 3. Generate proofs
    // 4. Submit results

    // Example: Fetch ready jobs from storage
    let ready_jobs = fetch_ready_jobs::<T>()?;
    
    log::info!("OCW: Found {} ready jobs", ready_jobs.len());

    for job_id in ready_jobs.iter().take(5) {
        // Execute job
        match execute_job::<T>(*job_id) {
            Ok(result) => {
                log::info!("OCW: Job {} executed successfully", job_id);
                
                // Generate proof
                let proof = generate_proof::<T>(&result)?;
                
                // Submit result
                submit_job_result::<T>(*job_id, result, proof)?;
            }
            Err(e) => {
                log::error!("OCW: Job {} execution failed: {:?}", job_id, e);
            }
        }
    }

    Ok(())
}

/// Fetch ready jobs from on-chain storage
fn fetch_ready_jobs<T: OffchainWorkerConfig>() -> Result<Vec<u64>, &'static str> {
    // In a real implementation, this would read from on-chain storage
    // For now, return empty vec
    Ok(Vec::new())
}

/// Execute a job
fn execute_job<T: OffchainWorkerConfig>(job_id: u64) -> Result<JobExecutionResult, &'static str> {
    log::info!("OCW: Executing job {}...", job_id);

    let start_time = sp_io::offchain::timestamp();

    // Simulate job execution
    // In a real implementation, this would:
    // 1. Fetch job metadata
    // 2. Parse job parameters
    // 3. Execute computation
    // 4. Collect results

    let result_data = perform_computation(job_id)?;

    let end_time = sp_io::offchain::timestamp();
    let execution_time_ms = end_time.diff(&start_time).millis();

    Ok(JobExecutionResult {
        job_id,
        result_data,
        execution_time_ms,
        success: true,
    })
}

/// Perform actual computation
fn perform_computation(job_id: u64) -> Result<Vec<u8>, &'static str> {
    log::info!("OCW: Computing for job {}...", job_id);

    // Example computations based on job type
    // In a real implementation, this could:
    // - Fetch data from HTTP APIs
    // - Perform complex calculations
    // - Aggregate data from multiple sources
    // - Run ML inference
    
    // For demonstration, create a simple result
    let mut result = Vec::new();
    result.extend_from_slice(b"Job result for ID: ");
    result.extend_from_slice(job_id.to_string().as_bytes());
    
    // Simulate some work
    sp_io::offchain::sleep_until(
        sp_io::offchain::timestamp().add(Duration::from_millis(100))
    );

    Ok(result)
}

/// Generate proof for job result
fn generate_proof<T: OffchainWorkerConfig>(
    result: &JobExecutionResult,
) -> Result<Vec<u8>, &'static str> {
    log::info!("OCW: Generating proof for job {}...", result.job_id);

    // In a real implementation, this would:
    // 1. Hash the result
    // 2. Sign with OCW key
    // 3. Or create Merkle proof
    // 4. Or generate ZK proof

    // For now, create a simple hash-based proof
    let result_hash = sp_io::hashing::blake2_256(&result.result_data);
    
    Ok(result_hash.to_vec())
}

/// Submit job result to chain
fn submit_job_result<T: OffchainWorkerConfig>(
    job_id: u64,
    result: JobExecutionResult,
    proof: Vec<u8>,
) -> Result<(), &'static str> {
    log::info!("OCW: Submitting result for job {}...", job_id);

    // Calculate result hash
    let result_hash = sp_core::H256(sp_io::hashing::blake2_256(&result.result_data));

    // Create signed transaction
    let signer = Signer::<T, T::AuthorityId>::all_accounts();
    if !signer.can_sign() {
        log::error!("OCW: No signing keys available");
        return Err("No signing keys available");
    }

    let results = signer.send_signed_transaction(|_account| {
        Call::SubmitJobResult {
            job_id,
            result_hash,
            proof_data: proof.clone(),
        }
    });

    for (acc, res) in &results {
        match res {
            Ok(()) => {
                log::info!("OCW: Successfully submitted result for job {}", job_id);
                return Ok(());
            }
            Err(e) => {
                log::error!("OCW: Failed to submit with account {:?}: {:?}", acc, e);
            }
        }
    }

    Err("Failed to submit job result")
}

/// Process pending events
fn process_pending_events<T: OffchainWorkerConfig>(
    block_number: T::BlockNumber,
) -> Result<(), &'static str> {
    log::info!("OCW: Processing pending events...");

    // Fetch pending events
    let pending_events = fetch_pending_events::<T>()?;
    
    log::info!("OCW: Found {} pending events", pending_events.len());

    for event_id in pending_events.iter().take(3) {
        // Process event
        let signer = Signer::<T, T::AuthorityId>::all_accounts();
        if !signer.can_sign() {
            log::warn!("OCW: No signing keys for event processing");
            continue;
        }

        let results = signer.send_signed_transaction(|_account| {
            Call::ProcessEvent {
                event_id: *event_id,
            }
        });

        for (acc, res) in &results {
            match res {
                Ok(()) => {
                    log::info!("OCW: Successfully processed event {}", event_id);
                    break;
                }
                Err(e) => {
                    log::error!("OCW: Failed to process event with account {:?}: {:?}", acc, e);
                }
            }
        }
    }

    Ok(())
}

/// Fetch pending events
fn fetch_pending_events<T: OffchainWorkerConfig>() -> Result<Vec<u64>, &'static str> {
    // Would read from on-chain storage
    Ok(Vec::new())
}

/// Collect and submit metrics
fn collect_metrics<T: OffchainWorkerConfig>(
    block_number: T::BlockNumber,
) -> Result<(), &'static str> {
    log::debug!("OCW: Collecting metrics at block {:?}...", block_number);

    // In a real implementation:
    // - Collect job execution stats
    // - Monitor validator performance
    // - Track consensus metrics
    // - Export to Prometheus

    Ok(())
}

/// HTTP fetching example (for external data)
fn fetch_external_data(url: &str) -> Result<Vec<u8>, http::Error> {
    log::info!("OCW: Fetching data from {}", url);

    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(10_000));
    
    let request = http::Request::get(url);
    let pending = request
        .deadline(deadline)
        .send()
        .map_err(|_| http::Error::IoError)?;

    let response = pending
        .try_wait(deadline)
        .map_err(|_| http::Error::DeadlineReached)??;

    if response.code != 200 {
        log::error!("OCW: HTTP request failed with code: {}", response.code);
        return Err(http::Error::Unknown);
    }

    Ok(response.body().collect::<Vec<u8>>())
}

/// Store value in offchain storage
fn store_offchain<T: OffchainWorkerConfig>(key: &[u8], value: &[u8]) {
    let storage = StorageValueRef::persistent(key);
    storage.set(value);
}

/// Load value from offchain storage
fn load_offchain<T: OffchainWorkerConfig>(key: &[u8]) -> Option<Vec<u8>> {
    let storage = StorageValueRef::persistent(key);
    storage.get().ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_execution_result() {
        let result = JobExecutionResult {
            job_id: 1,
            result_data: vec![1, 2, 3],
            execution_time_ms: 100,
            success: true,
        };

        assert_eq!(result.job_id, 1);
        assert!(result.success);
    }
}
