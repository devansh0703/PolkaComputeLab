//! Benchmarking setup for pallet-event-hub

use super::*;

#[allow(unused)]
use crate::Pallet as EventHub;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    impl_benchmark_test_suite!(EventHub, crate::mock::new_test_ext(), crate::mock::Test);
}
