//! Benchmarking setup for pallet-telemetry

use super::*;

#[allow(unused)]
use crate::Pallet as Telemetry;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    impl_benchmark_test_suite!(Telemetry, crate::mock::new_test_ext(), crate::mock::Test);
}
