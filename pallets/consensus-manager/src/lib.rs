#![cfg_attr(not(feature = "std"), no_std)]

//! # Consensus Manager Pallet
//!
//! This pallet manages consensus algorithm switching and collects block metrics.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Consensus algorithm types
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ConsensusType {
        /// Aura consensus
        Aura,
        /// BABE consensus  
        Babe,
        /// Custom consensus (for experimentation)
        Custom,
    }

    impl Default for ConsensusType {
        fn default() -> Self {
            ConsensusType::Aura
        }
    }

    /// Block metrics structure
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct BlockMetrics {
        /// Block number
        pub block_number: u32,
        /// Number of validators in set
        pub validator_count: u32,
        /// Block production time (milliseconds)
        pub block_time_ms: u64,
        /// Number of forks detected
        pub fork_count: u32,
        /// Consensus type at this block
        pub consensus_type: ConsensusType,
    }

    /// Validator performance metrics
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct ValidatorMetrics {
        /// Blocks produced
        pub blocks_produced: u32,
        /// Blocks missed
        pub blocks_missed: u32,
        /// Participation rate (percentage)
        pub participation_rate: u8,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of validator metrics to store
        #[pallet::constant]
        type MaxValidators: Get<u32>;
    }

    /// Current consensus type
    #[pallet::storage]
    #[pallet::getter(fn consensus_type)]
    pub type CurrentConsensus<T: Config> = StorageValue<_, ConsensusType, ValueQuery>;

    /// Block metrics history (limited to recent blocks)
    #[pallet::storage]
    #[pallet::getter(fn block_metrics)]
    pub type BlockMetricsHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BlockMetrics,
    >;

    /// Validator performance metrics
    #[pallet::storage]
    #[pallet::getter(fn validator_metrics)]
    pub type ValidatorPerformance<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ValidatorMetrics,
        ValueQuery,
    >;

    /// Fork statistics
    #[pallet::storage]
    #[pallet::getter(fn fork_stats)]
    pub type ForkStatistics<T: Config> = StorageValue<
        _,
        ForkStats,
        ValueQuery,
    >;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct ForkStats {
        pub total_forks: u32,
        pub last_fork_block: u32,
    }

    /// Consensus switch history
    #[pallet::storage]
    #[pallet::getter(fn consensus_switches)]
    pub type ConsensusSwitches<T: Config> = StorageValue<
        _,
        BoundedVec<(u32, ConsensusType), ConstU32<100>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Consensus type changed [block_number, old_type, new_type]
        ConsensusChanged {
            block_number: u32,
            old_type: ConsensusType,
            new_type: ConsensusType,
        },
        /// Block metrics recorded [block_number]
        BlockMetricsRecorded { block_number: u32 },
        /// Fork detected [block_number, fork_count]
        ForkDetected { block_number: u32, fork_count: u32 },
        /// Validator metrics updated [validator, blocks_produced]
        ValidatorMetricsUpdated {
            validator: T::AccountId,
            blocks_produced: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid consensus type
        InvalidConsensusType,
        /// Metrics not found
        MetricsNotFound,
        /// Too many validators
        TooManyValidators,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Record metrics at the end of each block
        fn on_finalize(block_number: BlockNumberFor<T>) {
            let bn: u32 = block_number.saturated_into();
            
            // Record block metrics
            let metrics = BlockMetrics {
                block_number: bn,
                validator_count: 0, // Would be populated from actual validator set
                block_time_ms: 6000, // Default block time
                fork_count: 0,
                consensus_type: CurrentConsensus::<T>::get(),
            };

            BlockMetricsHistory::<T>::insert(bn, metrics);
            Self::deposit_event(Event::BlockMetricsRecorded { block_number: bn });

            // Cleanup old metrics (keep last 1000 blocks)
            if bn > 1000_u32 {
                BlockMetricsHistory::<T>::remove(bn - 1000_u32);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set consensus type (admin only)
        ///
        /// # Parameters
        /// - `origin`: Root origin (sudo)
        /// - `consensus_type`: The new consensus type
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_consensus())]
        pub fn set_consensus(
            origin: OriginFor<T>,
            consensus_type: ConsensusType,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let old_type = CurrentConsensus::<T>::get();
            let block_number: u32 = frame_system::Pallet::<T>::block_number().saturated_into();

            // Update consensus type
            CurrentConsensus::<T>::put(consensus_type.clone());

            // Record the switch
            ConsensusSwitches::<T>::mutate(|switches| {
                let _ = switches.try_push((block_number, consensus_type.clone()));
            });

            Self::deposit_event(Event::ConsensusChanged {
                block_number,
                old_type,
                new_type: consensus_type,
            });

            Ok(())
        }

        /// Record block metrics manually (for testing/benchmarking)
        ///
        /// # Parameters
        /// - `origin`: Root origin
        /// - `block_number`: Block to record metrics for
        /// - `validator_count`: Number of validators
        /// - `block_time_ms`: Block production time
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::record_metrics())]
        pub fn record_metrics(
            origin: OriginFor<T>,
            block_number: u32,
            validator_count: u32,
            block_time_ms: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let metrics = BlockMetrics {
                block_number,
                validator_count,
                block_time_ms,
                fork_count: 0,
                consensus_type: CurrentConsensus::<T>::get(),
            };

            BlockMetricsHistory::<T>::insert(block_number, metrics);
            Self::deposit_event(Event::BlockMetricsRecorded { block_number });

            Ok(())
        }

        /// Record a fork detection
        ///
        /// # Parameters
        /// - `origin`: Root or OCW
        /// - `block_number`: Block where fork was detected
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::record_fork())]
        pub fn record_fork(
            origin: OriginFor<T>,
            block_number: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ForkStatistics::<T>::mutate(|stats| {
                stats.total_forks = stats.total_forks.saturating_add(1);
                stats.last_fork_block = block_number;
            });

            let fork_count = ForkStatistics::<T>::get().total_forks;

            Self::deposit_event(Event::ForkDetected { block_number, fork_count });

            Ok(())
        }

        /// Update validator metrics
        ///
        /// # Parameters
        /// - `origin`: Root or validator
        /// - `validator`: Validator account
        /// - `blocks_produced`: Number of blocks produced
        /// - `blocks_missed`: Number of blocks missed
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_validator_metrics())]
        pub fn update_validator_metrics(
            origin: OriginFor<T>,
            validator: T::AccountId,
            blocks_produced: u32,
            blocks_missed: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let total_blocks = blocks_produced.saturating_add(blocks_missed);
            let participation_rate = if total_blocks > 0 {
                ((blocks_produced as u64 * 100) / total_blocks as u64) as u8
            } else {
                0
            };

            let metrics = ValidatorMetrics {
                blocks_produced,
                blocks_missed,
                participation_rate,
            };

            ValidatorPerformance::<T>::insert(&validator, metrics);

            Self::deposit_event(Event::ValidatorMetricsUpdated {
                validator,
                blocks_produced,
            });

            Ok(())
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Get current consensus type
        pub fn get_consensus_type() -> ConsensusType {
            CurrentConsensus::<T>::get()
        }

        /// Get metrics for a specific block
        pub fn get_block_metrics(block_number: u32) -> Option<BlockMetrics> {
            BlockMetricsHistory::<T>::get(block_number)
        }

        /// Get fork statistics
        pub fn get_fork_stats() -> ForkStats {
            ForkStatistics::<T>::get()
        }

        /// Get validator performance
        pub fn get_validator_performance(validator: &T::AccountId) -> ValidatorMetrics {
            ValidatorPerformance::<T>::get(validator)
        }

        /// Get consensus switch history
        pub fn get_consensus_history() -> Vec<(u32, ConsensusType)> {
            ConsensusSwitches::<T>::get().to_vec()
        }
    }
}
