#![cfg_attr(not(feature = "std"), no_std)]

//! # Telemetry Pallet
//!
//! This pallet collects and exposes metrics for jobs, consensus, and validators.
//! Metrics can be exported to Prometheus for monitoring and analysis.

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
    use sp_runtime::traits::SaturatedConversion;
    use pallet_job_registry::{JobStatus, Pallet as JobRegistry};
    use pallet_consensus_manager::{ConsensusType, Pallet as ConsensusManager};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Job execution metrics
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct JobMetrics {
        /// Job ID
        pub job_id: u64,
        /// Start block
        pub start_block: u32,
        /// End block (if completed)
        pub end_block: Option<u32>,
        /// Execution time in blocks
        pub execution_time_blocks: u32,
        /// Job status
        pub status: JobStatus,
        /// Success/failure
        pub succeeded: bool,
    }

    /// Validator performance metrics
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct ValidatorPerformance {
        /// Blocks produced
        pub blocks_produced: u64,
        /// Blocks missed
        pub blocks_missed: u64,
        /// Uptime percentage
        pub uptime_percentage: u8,
        /// Last block produced
        pub last_block_produced: u32,
    }

    /// System-wide metrics
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct SystemMetrics {
        /// Total jobs submitted
        pub total_jobs_submitted: u64,
        /// Total jobs completed
        pub total_jobs_completed: u64,
        /// Total jobs failed
        pub total_jobs_failed: u64,
        /// Total jobs verified
        pub total_jobs_verified: u64,
        /// Average job execution time (blocks)
        pub avg_job_execution_time: u32,
        /// Total forks detected
        pub total_forks: u32,
        /// Total consensus switches
        pub total_consensus_switches: u32,
        /// Current consensus type
        pub current_consensus: ConsensusType,
    }

    /// Block performance metrics
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct BlockPerformance {
        /// Block number
        pub block_number: u32,
        /// Block time in milliseconds
        pub block_time_ms: u64,
        /// Number of extrinsics
        pub extrinsic_count: u32,
        /// Block weight used
        pub weight_used: u64,
        /// Block size in bytes
        pub block_size_bytes: u32,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_job_registry::Config + pallet_consensus_manager::Config {
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of job metrics to store
        #[pallet::constant]
        type MaxJobMetrics: Get<u32>;
    }

    /// Job metrics history
    #[pallet::storage]
    #[pallet::getter(fn job_metrics)]
    pub type JobMetricsHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64, // job_id
        JobMetrics,
    >;

    /// Validator performance map
    #[pallet::storage]
    #[pallet::getter(fn validator_performance)]
    pub type ValidatorPerformanceMap<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ValidatorPerformance,
        ValueQuery,
    >;

    /// System-wide metrics
    #[pallet::storage]
    #[pallet::getter(fn system_metrics)]
    pub type SystemMetricsStorage<T: Config> = StorageValue<
        _,
        SystemMetrics,
        ValueQuery,
    >;

    /// Block performance history (last N blocks)
    #[pallet::storage]
    #[pallet::getter(fn block_performance)]
    pub type BlockPerformanceHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // block_number
        BlockPerformance,
    >;

    /// Job execution time samples (for average calculation)
    #[pallet::storage]
    #[pallet::getter(fn execution_time_samples)]
    pub type ExecutionTimeSamples<T: Config> = StorageValue<
        _,
        BoundedVec<u32, ConstU32<1000>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Job metrics recorded [job_id, execution_time]
        JobMetricsRecorded { job_id: u64, execution_time_blocks: u32 },
        /// Validator performance updated [validator, blocks_produced]
        ValidatorPerformanceUpdated { validator: T::AccountId, blocks_produced: u64 },
        /// System metrics updated
        SystemMetricsUpdated,
        /// Block performance recorded [block_number, block_time_ms]
        BlockPerformanceRecorded { block_number: u32, block_time_ms: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Job not found
        JobNotFound,
        /// Metrics not found
        MetricsNotFound,
        /// Overflow in calculation
        ArithmeticOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Update metrics at the end of each block
        fn on_finalize(block_number: BlockNumberFor<T>) {
            let bn: u32 = block_number.saturated_into();
            
            // Record block performance
            let block_perf = BlockPerformance {
                block_number: bn,
                block_time_ms: 12000, // Would be actual measured time
                extrinsic_count: 0, // Would count actual extrinsics
                weight_used: 0, // Would track actual weight
                block_size_bytes: 0, // Would track actual size
            };
            
            BlockPerformanceHistory::<T>::insert(bn, block_perf);

            // Cleanup old block performance data (keep last 1000 blocks)
            if bn > 1000_u32 {
                BlockPerformanceHistory::<T>::remove(bn - 1000_u32);
            }

            // Update system metrics
            Self::update_system_metrics();

            Self::deposit_event(Event::BlockPerformanceRecorded {
                block_number: bn,
                block_time_ms: 12000,
            });
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Record job metrics
        ///
        /// # Parameters
        /// - `origin`: Root or authorized account
        /// - `job_id`: Job ID
        /// - `start_block`: Start block number
        /// - `end_block`: End block number (if completed)
        /// - `succeeded`: Whether job succeeded
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::record_job_metrics())]
        pub fn record_job_metrics(
            origin: OriginFor<T>,
            job_id: u64,
            start_block: u32,
            end_block: Option<u32>,
            succeeded: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let execution_time = if let Some(end) = end_block {
                end.saturating_sub(start_block)
            } else {
                0
            };

            let job = JobRegistry::<T>::jobs(job_id).ok_or(Error::<T>::JobNotFound)?;

            let metrics = JobMetrics {
                job_id,
                start_block,
                end_block,
                execution_time_blocks: execution_time,
                status: job.status,
                succeeded,
            };

            JobMetricsHistory::<T>::insert(job_id, metrics);

            // Add to execution time samples
            if execution_time > 0 {
                ExecutionTimeSamples::<T>::try_mutate(|samples| -> DispatchResult {
                    let _ = samples.try_push(execution_time);
                    // Keep only last 1000 samples
                    if samples.len() > 1000 {
                        samples.remove(0);
                    }
                    Ok(())
                })?;
            }

            Self::deposit_event(Event::JobMetricsRecorded {
                job_id,
                execution_time_blocks: execution_time,
            });

            Ok(())
        }

        /// Update validator performance
        ///
        /// # Parameters
        /// - `origin`: Root
        /// - `validator`: Validator account
        /// - `blocks_produced`: Blocks produced
        /// - `blocks_missed`: Blocks missed
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::update_validator_performance())]
        pub fn update_validator_performance(
            origin: OriginFor<T>,
            validator: T::AccountId,
            blocks_produced: u64,
            blocks_missed: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let total_blocks = blocks_produced.saturating_add(blocks_missed);
            let uptime = if total_blocks > 0 {
                ((blocks_produced * 100) / total_blocks) as u8
            } else {
                0
            };

            let performance = ValidatorPerformance {
                blocks_produced,
                blocks_missed,
                uptime_percentage: uptime,
                last_block_produced: frame_system::Pallet::<T>::block_number().saturated_into(),
            };

            ValidatorPerformanceMap::<T>::insert(&validator, performance);

            Self::deposit_event(Event::ValidatorPerformanceUpdated {
                validator,
                blocks_produced,
            });

            Ok(())
        }

        /// Manually trigger system metrics update
        ///
        /// # Parameters
        /// - `origin`: Anyone can trigger (typically OCW)
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::update_system_metrics())]
        pub fn trigger_system_metrics_update(
            origin: OriginFor<T>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            Self::update_system_metrics();
            Ok(())
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Update system-wide metrics
        fn update_system_metrics() {
            let mut metrics = SystemMetricsStorage::<T>::get();

            // Count jobs by status
            let _submitted = 0u64;
            let _completed = 0u64;
            let _failed = 0u64;
            let _verified = 0u64;

            // This would iterate through job registry in a real implementation
            // For now, we'll use existing stats if available
            
            // Get consensus info
            metrics.current_consensus = ConsensusManager::<T>::get_consensus_type();
            
            // Get fork stats
            let fork_stats = ConsensusManager::<T>::get_fork_stats();
            metrics.total_forks = fork_stats.total_forks;

            // Calculate average execution time
            let samples = ExecutionTimeSamples::<T>::get();
            if !samples.is_empty() {
                let sum: u64 = samples.iter().map(|&x| x as u64).sum();
                metrics.avg_job_execution_time = (sum / samples.len() as u64) as u32;
            }

            // Get consensus switch count
            let consensus_history = ConsensusManager::<T>::get_consensus_history();
            metrics.total_consensus_switches = consensus_history.len() as u32;

            SystemMetricsStorage::<T>::put(metrics);
            Self::deposit_event(Event::SystemMetricsUpdated);
        }

        /// Get job metrics
        pub fn get_job_metrics(job_id: u64) -> Option<JobMetrics> {
            JobMetricsHistory::<T>::get(job_id)
        }

        /// Get system metrics
        pub fn get_system_metrics() -> SystemMetrics {
            SystemMetricsStorage::<T>::get()
        }

        /// Get validator performance
        pub fn get_validator_performance(validator: &T::AccountId) -> ValidatorPerformance {
            ValidatorPerformanceMap::<T>::get(validator)
        }

        /// Get block performance
        pub fn get_block_performance(block_number: u32) -> Option<BlockPerformance> {
            BlockPerformanceHistory::<T>::get(block_number)
        }

        /// Get average job execution time
        pub fn get_average_execution_time() -> u32 {
            let samples = ExecutionTimeSamples::<T>::get();
            if samples.is_empty() {
                return 0;
            }
            
            let sum: u64 = samples.iter().map(|&x| x as u64).sum();
            (sum / samples.len() as u64) as u32
        }

        /// Export metrics in Prometheus format (for std environment)
        #[cfg(feature = "std")]
        pub fn export_prometheus_metrics() -> String {
            let metrics = Self::get_system_metrics();
            
            format!(
                "# HELP polkacomputelab_jobs_submitted_total Total jobs submitted\n\
                 # TYPE polkacomputelab_jobs_submitted_total counter\n\
                 polkacomputelab_jobs_submitted_total {}\n\
                 \n\
                 # HELP polkacomputelab_jobs_completed_total Total jobs completed\n\
                 # TYPE polkacomputelab_jobs_completed_total counter\n\
                 polkacomputelab_jobs_completed_total {}\n\
                 \n\
                 # HELP polkacomputelab_jobs_failed_total Total jobs failed\n\
                 # TYPE polkacomputelab_jobs_failed_total counter\n\
                 polkacomputelab_jobs_failed_total {}\n\
                 \n\
                 # HELP polkacomputelab_jobs_verified_total Total jobs verified\n\
                 # TYPE polkacomputelab_jobs_verified_total counter\n\
                 polkacomputelab_jobs_verified_total {}\n\
                 \n\
                 # HELP polkacomputelab_avg_job_execution_time_blocks Average job execution time in blocks\n\
                 # TYPE polkacomputelab_avg_job_execution_time_blocks gauge\n\
                 polkacomputelab_avg_job_execution_time_blocks {}\n\
                 \n\
                 # HELP polkacomputelab_forks_detected_total Total forks detected\n\
                 # TYPE polkacomputelab_forks_detected_total counter\n\
                 polkacomputelab_forks_detected_total {}\n\
                 \n\
                 # HELP polkacomputelab_consensus_switches_total Total consensus switches\n\
                 # TYPE polkacomputelab_consensus_switches_total counter\n\
                 polkacomputelab_consensus_switches_total {}\n",
                metrics.total_jobs_submitted,
                metrics.total_jobs_completed,
                metrics.total_jobs_failed,
                metrics.total_jobs_verified,
                metrics.avg_job_execution_time,
                metrics.total_forks,
                metrics.total_consensus_switches,
            )
        }
    }
}
