//! Weights for pallet_consensus_manager

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn set_consensus() -> Weight;
    fn record_metrics() -> Weight;
    fn record_fork() -> Weight;
    fn update_validator_metrics() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn set_consensus() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn record_metrics() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn record_fork() -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn update_validator_metrics() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn set_consensus() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn record_metrics() -> Weight {
        Weight::from_parts(15_000_000, 0)
    }

    fn record_fork() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }

    fn update_validator_metrics() -> Weight {
        Weight::from_parts(15_000_000, 0)
    }
}
