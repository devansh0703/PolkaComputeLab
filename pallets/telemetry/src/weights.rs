//! Weights for pallet_telemetry

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn record_job_metrics() -> Weight;
    fn update_validator_performance() -> Weight;
    fn update_system_metrics() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn record_job_metrics() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn update_validator_performance() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn update_system_metrics() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(5))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn record_job_metrics() -> Weight {
        Weight::from_parts(25_000_000, 0)
    }

    fn update_validator_performance() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }

    fn update_system_metrics() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }
}
