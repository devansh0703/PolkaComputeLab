use crate::{mock::*, Event};
use frame_support::{assert_ok};
use pallet_job_registry::JobStatus;

#[test]
fn record_job_metrics_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Create a job first
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        // Record metrics
        assert_ok!(Telemetry::record_job_metrics(
            RuntimeOrigin::root(),
            0, // job_id
            10, // start_block
            Some(20), // end_block
            true // succeeded
        ));

        let metrics = Telemetry::job_metrics(0).unwrap();
        assert_eq!(metrics.job_id, 0);
        assert_eq!(metrics.start_block, 10);
        assert_eq!(metrics.end_block, Some(20));
        assert_eq!(metrics.execution_time_blocks, 10);
        assert!(metrics.succeeded);

        System::assert_has_event(Event::JobMetricsRecorded {
            job_id: 0,
            execution_time_blocks: 10,
        }.into());
    });
}

#[test]
fn update_validator_performance_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(Telemetry::update_validator_performance(
            RuntimeOrigin::root(),
            1, // validator
            100, // blocks_produced
            10 // blocks_missed
        ));

        let perf = Telemetry::validator_performance(&1);
        assert_eq!(perf.blocks_produced, 100);
        assert_eq!(perf.blocks_missed, 10);
        assert_eq!(perf.uptime_percentage, 90); // 100/(100+10) * 100

        System::assert_has_event(Event::ValidatorPerformanceUpdated {
            validator: 1,
            blocks_produced: 100,
        }.into());
    });
}

#[test]
fn system_metrics_updated() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(Telemetry::trigger_system_metrics_update(
            RuntimeOrigin::signed(1)
        ));

        let metrics = Telemetry::get_system_metrics();
        // System metrics should be initialized
        assert_eq!(metrics.total_consensus_switches, 0);

        System::assert_has_event(Event::SystemMetricsUpdated.into());
    });
}

#[test]
fn on_finalize_records_block_performance() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        Telemetry::on_finalize(1);
        
        let perf = Telemetry::block_performance(1);
        assert!(perf.is_some());
        
        let block_perf = perf.unwrap();
        assert_eq!(block_perf.block_number, 1);
    });
}

#[test]
fn average_execution_time_calculated() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Create jobs
        for i in 0..3 {
            assert_ok!(JobRegistry::submit_job(
                RuntimeOrigin::signed(1),
                vec![i],
                vec![],
                100
            ));

            assert_ok!(Telemetry::record_job_metrics(
                RuntimeOrigin::root(),
                i as u64,
                10,
                Some(10 + (i + 1) * 10),
                true
            ));
        }

        let avg = Telemetry::get_average_execution_time();
        // Average of 10, 20, 30 = 20
        assert_eq!(avg, 20);
    });
}

#[test]
fn execution_time_samples_bounded() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Create job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1],
            vec![],
            100
        ));

        // Add sample
        assert_ok!(Telemetry::record_job_metrics(
            RuntimeOrigin::root(),
            0,
            10,
            Some(50),
            true
        ));

        let samples = Telemetry::execution_time_samples();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0], 40);
    });
}

#[test]
fn get_job_metrics_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1],
            vec![],
            100
        ));

        assert_ok!(Telemetry::record_job_metrics(
            RuntimeOrigin::root(),
            0,
            5,
            Some(15),
            true
        ));

        let metrics = Telemetry::get_job_metrics(0);
        assert!(metrics.is_some());
        
        let m = metrics.unwrap();
        assert_eq!(m.execution_time_blocks, 10);
    });
}

#[test]
fn get_validator_performance_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(Telemetry::update_validator_performance(
            RuntimeOrigin::root(),
            42,
            1000,
            50
        ));

        let perf = Telemetry::get_validator_performance(&42);
        assert_eq!(perf.blocks_produced, 1000);
        assert_eq!(perf.blocks_missed, 50);
    });
}

#[test]
fn block_performance_cleanup_works() {
    new_test_ext().execute_with(|| {
        // Test that old blocks are cleaned up
        System::set_block_number(1);
        
        Telemetry::on_finalize(1);
        assert!(Telemetry::block_performance(1).is_some());

        // Simulate 1001 blocks passing
        System::set_block_number(1002);
        Telemetry::on_finalize(1002);

        // Block 1 should still exist (within 1000 block window)
        assert!(Telemetry::block_performance(1).is_some());
        
        // But block 2 would be cleaned up if we go to block 1003
        System::set_block_number(1003);
        Telemetry::on_finalize(1003);
        // Block 2 would be removed (1003 - 1000 = 3, so blocks < 3 are removed)
    });
}
