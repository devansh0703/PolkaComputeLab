//! Benchmarking setup for pallet-job-registry

use super::*;

#[allow(unused)]
use crate::Pallet as JobRegistry;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn submit_job() {
        let caller: T::AccountId = whitelisted_caller();
        let metadata = vec![1u8; 256];
        let dependencies = vec![];
        let deadline = 1000u32.into();

        #[extrinsic_call]
        submit_job(RawOrigin::Signed(caller), metadata, dependencies, deadline);

        assert_eq!(NextJobId::<T>::get(), 1);
    }

    #[benchmark]
    fn update_job_status() {
        let caller: T::AccountId = whitelisted_caller();
        let metadata = vec![1u8; 256];
        
        // Setup: create a job
        let _ = JobRegistry::<T>::submit_job(
            RawOrigin::Signed(caller.clone()).into(),
            metadata,
            vec![],
            1000u32.into(),
        );

        #[extrinsic_call]
        update_job_status(RawOrigin::Signed(caller), 0, JobStatus::InProgress);

        assert!(Jobs::<T>::get(0).is_some());
    }

    #[benchmark]
    fn remove_job() {
        let caller: T::AccountId = whitelisted_caller();
        let metadata = vec![1u8; 256];
        
        // Setup: create and complete a job
        let _ = JobRegistry::<T>::submit_job(
            RawOrigin::Signed(caller.clone()).into(),
            metadata,
            vec![],
            1000u32.into(),
        );
        let _ = JobRegistry::<T>::update_job_status(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            JobStatus::InProgress,
        );
        let _ = JobRegistry::<T>::update_job_status(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            JobStatus::Completed,
        );

        #[extrinsic_call]
        remove_job(RawOrigin::Signed(caller), 0);

        assert!(Jobs::<T>::get(0).is_none());
    }

    impl_benchmark_test_suite!(JobRegistry, crate::mock::new_test_ext(), crate::mock::Test);
}
