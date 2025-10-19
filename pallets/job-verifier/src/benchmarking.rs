//! Benchmarking setup for pallet-job-verifier

use super::*;

#[allow(unused)]
use crate::Pallet as JobVerifier;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    impl_benchmark_test_suite!(JobVerifier, crate::mock::new_test_ext(), crate::mock::Test);
}
