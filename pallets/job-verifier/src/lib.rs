#![cfg_attr(not(feature = "std"), no_std)]

//! # Job Verifier Pallet
//!
//! This pallet verifies off-chain job results submitted by OCWs.
//! It supports signature-based verification and Merkle proof validation.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::vec::Vec;
    use pallet_job_registry::{JobStatus, Pallet as JobRegistry};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Proof type enumeration
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ProofType {
        /// Simple signature-based proof
        Signature,
        /// Merkle tree root proof
        MerkleRoot,
        /// Hash-based proof
        Hash,
    }

    /// Job result structure
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct JobResult {
        /// Hash of the result
        pub result_hash: H256,
        /// Proof type used
        pub proof_type: ProofType,
        /// Block number when result was submitted
        pub submitted_at: u32,
        /// Whether the result has been verified
        pub verified: bool,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_job_registry::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        /// Maximum size of proof data
        #[pallet::constant]
        type MaxProofSize: Get<u32>;
    }

    /// Map from JobId to JobResult
    #[pallet::storage]
    #[pallet::getter(fn job_results)]
    pub type JobResults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        JobResult,
    >;

    /// Map from JobId to proof data (for verification)
    #[pallet::storage]
    #[pallet::getter(fn job_proofs)]
    pub type JobProofs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        BoundedVec<u8, T::MaxProofSize>,
    >;

    /// Statistics for verification
    #[pallet::storage]
    #[pallet::getter(fn verification_stats)]
    pub type VerificationStats<T: Config> = StorageValue<
        _,
        VerificationStatistics,
        ValueQuery,
    >;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct VerificationStatistics {
        pub total_proofs_submitted: u64,
        pub total_proofs_verified: u64,
        pub total_proofs_failed: u64,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Proof submitted for job [job_id, result_hash]
        ProofSubmitted { job_id: u64, result_hash: H256 },
        /// Job result verified [job_id]
        JobVerified { job_id: u64 },
        /// Job verification failed [job_id, reason]
        VerificationFailed { job_id: u64 },
        /// Proof data stored [job_id, proof_size]
        ProofStored { job_id: u64, proof_size: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Job not found
        JobNotFound,
        /// Job already verified
        AlreadyVerified,
        /// Invalid proof
        InvalidProof,
        /// Proof too large
        ProofTooLarge,
        /// Job not in correct status for verification
        InvalidJobStatus,
        /// Not authorized
        NotAuthorized,
        /// Result hash mismatch
        ResultHashMismatch,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a proof for a job result
        ///
        /// # Parameters
        /// - `origin`: The off-chain worker or authorized account
        /// - `job_id`: The job ID
        /// - `result_hash`: Hash of the computation result
        /// - `proof_type`: Type of proof being submitted
        /// - `proof_data`: The proof data (signature, merkle proof, etc.)
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_proof())]
        pub fn submit_proof(
            origin: OriginFor<T>,
            job_id: u64,
            result_hash: H256,
            proof_type: ProofType,
            proof_data: Vec<u8>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Check job exists
            let job = JobRegistry::<T>::jobs(job_id)
                .ok_or(Error::<T>::JobNotFound)?;

            // Check job is in correct status (InProgress or Completed)
            ensure!(
                matches!(job.status, JobStatus::InProgress | JobStatus::Completed),
                Error::<T>::InvalidJobStatus
            );

            // Check if already verified
            if let Some(result) = JobResults::<T>::get(job_id) {
                ensure!(!result.verified, Error::<T>::AlreadyVerified);
            }

            // Validate proof data size
            let bounded_proof: BoundedVec<u8, T::MaxProofSize> = proof_data
                .try_into()
                .map_err(|_| Error::<T>::ProofTooLarge)?;

            // Store proof data
            JobProofs::<T>::insert(job_id, bounded_proof.clone());

            // Create result entry
            let result = JobResult {
                result_hash,
                proof_type,
                submitted_at: frame_system::Pallet::<T>::block_number().saturated_into(),
                verified: false,
            };

            JobResults::<T>::insert(job_id, result);

            // Update statistics
            VerificationStats::<T>::mutate(|stats| {
                stats.total_proofs_submitted = stats.total_proofs_submitted.saturating_add(1);
            });

            Self::deposit_event(Event::ProofSubmitted { job_id, result_hash });
            Self::deposit_event(Event::ProofStored { 
                job_id, 
                proof_size: bounded_proof.len() as u32 
            });

            Ok(())
        }

        /// Verify a submitted proof
        ///
        /// # Parameters
        /// - `origin`: Sudo or authorized verifier
        /// - `job_id`: The job ID to verify
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::verify_proof())]
        pub fn verify_proof(
            origin: OriginFor<T>,
            job_id: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Get job
            let job = JobRegistry::<T>::jobs(job_id)
                .ok_or(Error::<T>::JobNotFound)?;

            // Get result
            let mut result = JobResults::<T>::get(job_id)
                .ok_or(Error::<T>::JobNotFound)?;

            // Check not already verified
            ensure!(!result.verified, Error::<T>::AlreadyVerified);

            // Get proof data
            let proof_data = JobProofs::<T>::get(job_id)
                .ok_or(Error::<T>::InvalidProof)?;

            // Verify based on proof type
            let verification_result = match result.proof_type {
                ProofType::Signature => Self::verify_signature(&job, &result, &proof_data),
                ProofType::MerkleRoot => Self::verify_merkle_proof(&result, &proof_data),
                ProofType::Hash => Self::verify_hash(&result, &proof_data),
            };

            if verification_result {
                // Mark as verified
                result.verified = true;
                JobResults::<T>::insert(job_id, result);

                // Update job status to Verified
                let _ = JobRegistry::<T>::update_job_status(
                    frame_system::RawOrigin::Signed(job.owner.clone()).into(),
                    job_id,
                    JobStatus::Verified,
                );

                // Update statistics
                VerificationStats::<T>::mutate(|stats| {
                    stats.total_proofs_verified = stats.total_proofs_verified.saturating_add(1);
                });

                Self::deposit_event(Event::JobVerified { job_id });
                Ok(())
            } else {
                // Update statistics
                VerificationStats::<T>::mutate(|stats| {
                    stats.total_proofs_failed = stats.total_proofs_failed.saturating_add(1);
                });

                Self::deposit_event(Event::VerificationFailed { job_id });
                Err(Error::<T>::InvalidProof.into())
            }
        }

        /// Mark a job as verified (for OCW or authorized accounts)
        ///
        /// # Parameters
        /// - `origin`: Root or authorized account
        /// - `job_id`: The job ID
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::mark_verified())]
        pub fn mark_verified(
            origin: OriginFor<T>,
            job_id: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            JobResults::<T>::try_mutate(job_id, |maybe_result| -> DispatchResult {
                let result = maybe_result.as_mut().ok_or(Error::<T>::JobNotFound)?;
                result.verified = true;

                // Update job status
                let job = JobRegistry::<T>::jobs(job_id)
                    .ok_or(Error::<T>::JobNotFound)?;

                let _ = JobRegistry::<T>::update_job_status(
                    frame_system::RawOrigin::Signed(job.owner.clone()).into(),
                    job_id,
                    JobStatus::Verified,
                );

                // Update statistics
                VerificationStats::<T>::mutate(|stats| {
                    stats.total_proofs_verified = stats.total_proofs_verified.saturating_add(1);
                });

                Self::deposit_event(Event::JobVerified { job_id });
                Ok(())
            })
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Verify signature-based proof
        fn verify_signature(
            _job: &pallet_job_registry::Job<T::AccountId, BlockNumberFor<T>>,
            _result: &JobResult,
            proof_data: &[u8],
        ) -> bool {
            // In a real implementation, this would verify a signature
            // For now, we do a simple check that proof data is non-empty
            // and matches expected format
            
            if proof_data.len() < 64 {
                return false;
            }

            // Signature verification would happen here
            // For demonstration, we accept if proof data is properly formatted
            true
        }

        /// Verify Merkle proof
        fn verify_merkle_proof(_result: &JobResult, proof_data: &[u8]) -> bool {
            // In a real implementation, this would verify a Merkle tree proof
            // For now, we check basic validity
            
            if proof_data.is_empty() {
                return false;
            }

            // Merkle proof verification would happen here
            // For demonstration, we accept if proof data exists
            true
        }

        /// Verify hash-based proof
        fn verify_hash(result: &JobResult, proof_data: &[u8]) -> bool {
            // Verify that the hash of proof data matches the result hash
            let computed_hash = sp_io::hashing::blake2_256(proof_data);
            let computed_h256 = H256::from(computed_hash);
            
            computed_h256 == result.result_hash
        }

        /// Check if a job has been verified
        pub fn is_verified(job_id: u64) -> bool {
            if let Some(result) = JobResults::<T>::get(job_id) {
                result.verified
            } else {
                false
            }
        }

        /// Get verification statistics
        pub fn get_stats() -> VerificationStatistics {
            VerificationStats::<T>::get()
        }
    }
}
