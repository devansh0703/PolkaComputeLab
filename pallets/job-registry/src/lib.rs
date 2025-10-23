#![cfg_attr(not(feature = "std"), no_std)]

//! # Job Registry Pallet
//!
//! This pallet manages job registration, tracking, dependencies, and status updates.
//! It provides the core functionality for job orchestration in PolkaComputeLab.

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
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Job status enumeration
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[codec(dumb_trait_bound)]
    pub enum JobStatus {
        Pending,
        InProgress,
        Completed,
        Verified,
        Failed,
    }

    impl Default for JobStatus {
        fn default() -> Self {
            JobStatus::Pending
        }
    }

    impl JobStatus {
        /// Convert from u8 representation
        pub fn from_u8(value: u8) -> Result<Self, ()> {
            match value {
                0 => Ok(JobStatus::Pending),
                1 => Ok(JobStatus::InProgress),
                2 => Ok(JobStatus::Completed),
                3 => Ok(JobStatus::Verified),
                4 => Ok(JobStatus::Failed),
                _ => Err(()),
            }
        }
    }

    /// Job structure containing all job metadata
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Job<AccountId, BlockNumber> {
        /// Job owner
        pub owner: AccountId,
        /// Job metadata (could be IPFS hash, job parameters, etc.)
        pub metadata: BoundedVec<u8, ConstU32<256>>,
        /// List of job IDs that must complete before this job
        pub dependencies: BoundedVec<u64, ConstU32<10>>,
        /// Block number deadline
        pub deadline: BlockNumber,
        /// Current job status
        pub status: JobStatus,
        /// Block number when job was submitted
        pub submitted_at: BlockNumber,
        /// Block number when job was completed (if applicable)
        pub completed_at: Option<BlockNumber>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of active jobs per account
        #[pallet::constant]
        type MaxJobsPerAccount: Get<u32>;

        /// Maximum job dependency depth
        #[pallet::constant]
        type MaxDependencyDepth: Get<u32>;
    }

    /// Counter for generating unique job IDs
    #[pallet::storage]
    #[pallet::getter(fn next_job_id)]
    pub type NextJobId<T> = StorageValue<_, u64, ValueQuery>;

    /// Map from JobId to Job
    #[pallet::storage]
    #[pallet::getter(fn jobs)]
    pub type Jobs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        Job<T::AccountId, BlockNumberFor<T>>,
    >;

    /// Map from AccountId to their job IDs
    #[pallet::storage]
    #[pallet::getter(fn account_jobs)]
    pub type AccountJobs<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u64, T::MaxJobsPerAccount>,
        ValueQuery,
    >;

    /// Map to track jobs by status for efficient querying
    #[pallet::storage]
    #[pallet::getter(fn jobs_by_status)]
    pub type JobsByStatus<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        JobStatus,
        BoundedVec<u64, ConstU32<1000>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new job was submitted [job_id, owner]
        JobSubmitted { job_id: u64, owner: T::AccountId },
        /// Job status was updated [job_id]
        JobStatusUpdated { job_id: u64 },
        /// Job was completed [job_id, block_number]
        JobCompleted { job_id: u64, block_number: BlockNumberFor<T> },
        /// Job failed [job_id, reason]
        JobFailed { job_id: u64 },
        /// Job was removed [job_id]
        JobRemoved { job_id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Job not found
        JobNotFound,
        /// Not authorized to modify this job
        NotAuthorized,
        /// Invalid job status value
        InvalidJobStatus,
        /// Invalid job status transition
        InvalidStatusTransition,
        /// Circular dependency detected
        CircularDependency,
        /// Dependency not found
        DependencyNotFound,
        /// Dependency not completed
        DependencyNotCompleted,
        /// Maximum jobs per account reached
        MaxJobsReached,
        /// Deadline in the past
        DeadlineInPast,
        /// Job metadata too large
        MetadataTooLarge,
        /// Too many dependencies
        TooManyDependencies,
        /// Maximum dependency depth exceeded
        MaxDependencyDepthExceeded,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a new job
        ///
        /// # Parameters
        /// - `origin`: The account submitting the job
        /// - `metadata`: Job metadata (parameters, IPFS hash, etc.)
        /// - `dependencies`: List of job IDs that must complete first
        /// - `deadline`: Block number by which job should complete
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_job())]
        pub fn submit_job(
            origin: OriginFor<T>,
            metadata: Vec<u8>,
            dependencies: Vec<u64>,
            deadline: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate metadata size
            let bounded_metadata: BoundedVec<u8, ConstU32<256>> = metadata
                .try_into()
                .map_err(|_| Error::<T>::MetadataTooLarge)?;

            // Validate dependencies
            let bounded_dependencies: BoundedVec<u64, ConstU32<10>> = dependencies
                .try_into()
                .map_err(|_| Error::<T>::TooManyDependencies)?;

            // Validate deadline
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > current_block, Error::<T>::DeadlineInPast);

            // Check dependencies exist and validate no circular dependencies
            for dep_id in bounded_dependencies.iter() {
                ensure!(Jobs::<T>::contains_key(dep_id), Error::<T>::DependencyNotFound);
                Self::check_circular_dependency(*dep_id, &bounded_dependencies)?;
            }

            // Check max jobs per account
            let mut account_job_list = AccountJobs::<T>::get(&who);
            ensure!(
                (account_job_list.len() as u32) < T::MaxJobsPerAccount::get(),
                Error::<T>::MaxJobsReached
            );

            // Generate new job ID
            let job_id = NextJobId::<T>::get();
            let next_id = job_id.checked_add(1).ok_or(Error::<T>::JobNotFound)?;
            NextJobId::<T>::put(next_id);

            // Create job
            let job = Job {
                owner: who.clone(),
                metadata: bounded_metadata,
                dependencies: bounded_dependencies,
                deadline,
                status: JobStatus::Pending,
                submitted_at: current_block,
                completed_at: None,
            };

            // Store job
            Jobs::<T>::insert(job_id, job);

            // Add to account jobs
            account_job_list.try_push(job_id).map_err(|_| Error::<T>::MaxJobsReached)?;
            AccountJobs::<T>::insert(&who, account_job_list);

            // Add to pending jobs
            let mut pending_jobs = JobsByStatus::<T>::get(JobStatus::Pending);
            let _ = pending_jobs.try_push(job_id);
            JobsByStatus::<T>::insert(JobStatus::Pending, pending_jobs);

            Self::deposit_event(Event::JobSubmitted { job_id, owner: who });

            Ok(())
        }

        /// Update job status
        ///
        /// # Parameters
        /// - `origin`: The account updating the job (must be owner or sudo)
        /// - `job_id`: The job to update
        /// - `new_status_u8`: The new status (0=Pending, 1=InProgress, 2=Completed, 3=Verified, 4=Failed)
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_job_status())]
        pub fn update_job_status(
            origin: OriginFor<T>,
            job_id: u64,
            new_status_u8: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Convert u8 to JobStatus
            let new_status = JobStatus::from_u8(new_status_u8)
                .map_err(|_| Error::<T>::InvalidJobStatus)?;

            Jobs::<T>::try_mutate(job_id, |maybe_job| -> DispatchResult {
                let job = maybe_job.as_mut().ok_or(Error::<T>::JobNotFound)?;

                // Check authorization (owner or system)
                ensure!(job.owner == who, Error::<T>::NotAuthorized);

                // Validate status transition
                Self::validate_status_transition(&job.status, &new_status)?;

                let old_status = job.status.clone();

                // Update job
                job.status = new_status.clone();

                if matches!(new_status, JobStatus::Completed | JobStatus::Verified) {
                    job.completed_at = Some(frame_system::Pallet::<T>::block_number());
                }

                // Update status index
                Self::update_job_status_index(job_id, &old_status, &new_status)?;

                Self::deposit_event(Event::JobStatusUpdated {
                    job_id,
                });

                if matches!(new_status, JobStatus::Completed) {
                    Self::deposit_event(Event::JobCompleted {
                        job_id,
                        block_number: frame_system::Pallet::<T>::block_number(),
                    });
                } else if matches!(new_status, JobStatus::Failed) {
                    Self::deposit_event(Event::JobFailed { job_id });
                }

                Ok(())
            })
        }

        /// Remove a completed or failed job
        ///
        /// # Parameters
        /// - `origin`: The job owner
        /// - `job_id`: The job to remove
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::remove_job())]
        pub fn remove_job(
            origin: OriginFor<T>,
            job_id: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

            // Only owner can remove
            ensure!(job.owner == who, Error::<T>::NotAuthorized);

            // Only remove completed/failed/verified jobs
            ensure!(
                matches!(job.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Verified),
                Error::<T>::InvalidStatusTransition
            );

            // Remove from storage
            Jobs::<T>::remove(job_id);

            // Remove from account jobs
            AccountJobs::<T>::mutate(&who, |jobs| {
                jobs.retain(|&id| id != job_id);
            });

            // Remove from status index
            JobsByStatus::<T>::mutate(&job.status, |jobs| {
                jobs.retain(|&id| id != job_id);
            });

            Self::deposit_event(Event::JobRemoved { job_id });

            Ok(())
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Check if adding this dependency would create a circular dependency
        fn check_circular_dependency(
            dep_id: u64,
            _new_dependencies: &BoundedVec<u64, ConstU32<10>>,
        ) -> DispatchResult {
            Self::check_dependency_depth(dep_id, 0)?;
            Ok(())
        }

        /// Recursively check dependency depth to prevent circular dependencies
        fn check_dependency_depth(job_id: u64, depth: u32) -> DispatchResult {
            ensure!(
                depth < T::MaxDependencyDepth::get(),
                Error::<T>::MaxDependencyDepthExceeded
            );

            if let Some(job) = Jobs::<T>::get(job_id) {
                for dep_id in job.dependencies.iter() {
                    Self::check_dependency_depth(*dep_id, depth + 1)?;
                }
            }

            Ok(())
        }

        /// Validate that a status transition is allowed
        fn validate_status_transition(
            old_status: &JobStatus,
            new_status: &JobStatus,
        ) -> DispatchResult {
            let valid = match (old_status, new_status) {
                (JobStatus::Pending, JobStatus::InProgress) => true,
                (JobStatus::InProgress, JobStatus::Completed) => true,
                (JobStatus::InProgress, JobStatus::Failed) => true,
                (JobStatus::Completed, JobStatus::Verified) => true,
                (JobStatus::Pending, JobStatus::Failed) => true,
                _ => false,
            };

            ensure!(valid, Error::<T>::InvalidStatusTransition);
            Ok(())
        }

        /// Update the job status index
        fn update_job_status_index(
            job_id: u64,
            old_status: &JobStatus,
            new_status: &JobStatus,
        ) -> DispatchResult {
            // Remove from old status
            JobsByStatus::<T>::mutate(old_status, |jobs| {
                jobs.retain(|&id| id != job_id);
            });

            // Add to new status
            JobsByStatus::<T>::mutate(new_status, |jobs| {
                let _ = jobs.try_push(job_id);
            });

            Ok(())
        }

        /// Check if all dependencies for a job are completed
        pub fn are_dependencies_met(job_id: u64) -> bool {
            if let Some(job) = Jobs::<T>::get(job_id) {
                for dep_id in job.dependencies.iter() {
                    if let Some(dep_job) = Jobs::<T>::get(dep_id) {
                        if !matches!(dep_job.status, JobStatus::Completed | JobStatus::Verified) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }

        /// Get all pending jobs that are ready to execute (dependencies met)
        pub fn get_ready_jobs() -> Vec<u64> {
            let pending_jobs = JobsByStatus::<T>::get(JobStatus::Pending);
            pending_jobs
                .iter()
                .filter(|&&job_id| Self::are_dependencies_met(job_id))
                .copied()
                .collect()
        }
    }
}
