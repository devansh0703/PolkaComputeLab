use crate::{mock::*, ConsensusType, Event};
use frame_support::{assert_ok};

#[test]
fn set_consensus_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(ConsensusManager::set_consensus(
            RuntimeOrigin::root(),
            ConsensusType::Babe
        ));

        assert_eq!(ConsensusManager::consensus_type(), ConsensusType::Babe);
        
        System::assert_has_event(Event::ConsensusChanged {
            block_number: 1,
            old_type: ConsensusType::Aura,
            new_type: ConsensusType::Babe,
        }.into());
    });
}

#[test]
fn record_metrics_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(ConsensusManager::record_metrics(
            RuntimeOrigin::root(),
            10,
            4,
            6000
        ));

        let metrics = ConsensusManager::block_metrics(10).unwrap();
        assert_eq!(metrics.block_number, 10);
        assert_eq!(metrics.validator_count, 4);
    });
}

#[test]
fn record_fork_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(ConsensusManager::record_fork(
            RuntimeOrigin::root(),
            100
        ));

        let stats = ConsensusManager::fork_stats();
        assert_eq!(stats.total_forks, 1);
        assert_eq!(stats.last_fork_block, 100);
    });
}

#[test]
fn on_finalize_records_metrics() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        ConsensusManager::on_finalize(1);
        
        assert!(ConsensusManager::block_metrics(1).is_some());
    });
}
