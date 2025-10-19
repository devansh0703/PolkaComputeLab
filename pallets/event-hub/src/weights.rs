//! Weights for pallet_event_hub

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn submit_event() -> Weight;
    fn register_trigger() -> Weight;
    fn process_event() -> Weight;
    fn deactivate_trigger() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn submit_event() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn register_trigger() -> Weight {
        Weight::from_parts(35_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(4))
    }

    fn process_event() -> Weight {
        Weight::from_parts(40_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(5))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn deactivate_trigger() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn submit_event() -> Weight {
        Weight::from_parts(30_000_000, 0)
    }

    fn register_trigger() -> Weight {
        Weight::from_parts(35_000_000, 0)
    }

    fn process_event() -> Weight {
        Weight::from_parts(40_000_000, 0)
    }

    fn deactivate_trigger() -> Weight {
        Weight::from_parts(20_000_000, 0)
    }
}
