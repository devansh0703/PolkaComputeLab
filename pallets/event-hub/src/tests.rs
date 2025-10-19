use crate::{mock::*, Event, EventType, TriggerAction};
use frame_support::{assert_ok, assert_noop};
use pallet_job_registry::JobStatus;

#[test]
fn submit_event_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        let payload = vec![1, 2, 3, 4];
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            payload.clone(),
            None
        ));

        let event = EventHub::events(0).unwrap();
        assert_eq!(event.payload.to_vec(), payload);
        assert!(!event.processed);

        System::assert_has_event(Event::EventSubmitted {
            event_id: 0,
            event_type: EventType::OnChain,
        }.into());
    });
}

#[test]
fn submit_cross_chain_event_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        let payload = vec![1, 2, 3];
        let source_para_id = 2000;
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::CrossChain,
            payload,
            Some(source_para_id)
        ));

        let event = EventHub::events(0).unwrap();
        assert_eq!(event.source_para_id, Some(source_para_id));

        System::assert_has_event(Event::CrossChainEventReceived {
            event_id: 0,
            source_para_id,
        }.into());
    });
}

#[test]
fn register_trigger_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit event first
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        // Register trigger
        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0, // event_id
            TriggerAction::Custom,
            None
        ));

        let trigger = EventHub::triggers(0).unwrap();
        assert_eq!(trigger.event_id, 0);
        assert!(trigger.active);

        System::assert_has_event(Event::TriggerRegistered {
            trigger_id: 0,
            event_id: 0,
            owner: 1,
        }.into());
    });
}

#[test]
fn register_trigger_with_condition_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        let condition = vec![1u8; 64];
        
        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0,
            TriggerAction::Custom,
            Some(condition.clone())
        ));

        let trigger = EventHub::triggers(0).unwrap();
        assert!(trigger.condition.is_some());
    });
}

#[test]
fn process_event_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit event
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        // Register trigger
        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0,
            TriggerAction::Custom,
            None
        ));

        // Process event
        assert_ok!(EventHub::process_event(
            RuntimeOrigin::signed(1),
            0
        ));

        let event = EventHub::events(0).unwrap();
        assert!(event.processed);

        System::assert_has_event(Event::EventProcessed { event_id: 0 }.into());
        System::assert_has_event(Event::TriggerActivated {
            trigger_id: 0,
            event_id: 0,
        }.into());
    });
}

#[test]
fn process_already_processed_event_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        assert_ok!(EventHub::process_event(
            RuntimeOrigin::signed(1),
            0
        ));

        // Try to process again
        assert_noop!(
            EventHub::process_event(
                RuntimeOrigin::signed(1),
                0
            ),
            crate::Error::<Test>::AlreadyProcessed
        );
    });
}

#[test]
fn deactivate_trigger_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0,
            TriggerAction::Custom,
            None
        ));

        assert_ok!(EventHub::deactivate_trigger(
            RuntimeOrigin::signed(1),
            0
        ));

        let trigger = EventHub::triggers(0).unwrap();
        assert!(!trigger.active);

        System::assert_has_event(Event::TriggerDeactivated { trigger_id: 0 }.into());
    });
}

#[test]
fn deactivate_trigger_unauthorized_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0,
            TriggerAction::Custom,
            None
        ));

        // Try to deactivate as different user
        assert_noop!(
            EventHub::deactivate_trigger(
                RuntimeOrigin::signed(2),
                0
            ),
            crate::Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn trigger_starts_job() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Create a job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        // Submit event
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1, 2, 3],
            None
        ));

        // Register trigger to start the job
        assert_ok!(EventHub::register_trigger(
            RuntimeOrigin::signed(1),
            0, // event_id
            TriggerAction::StartJob(0), // job_id
            None
        ));

        // Process event
        assert_ok!(EventHub::process_event(
            RuntimeOrigin::signed(1),
            0
        ));

        // Check job status changed
        let job = JobRegistry::jobs(0).unwrap();
        assert_eq!(job.status, JobStatus::InProgress);

        System::assert_has_event(Event::JobTriggered {
            job_id: 0,
            event_id: 0,
        }.into());
    });
}

#[test]
fn get_pending_events_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit multiple events
        for i in 0..3 {
            assert_ok!(EventHub::submit_event(
                RuntimeOrigin::signed(1),
                EventType::OnChain,
                vec![i],
                None
            ));
        }

        let pending = EventHub::get_pending_events();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending, vec![0, 1, 2]);

        // Process one
        assert_ok!(EventHub::process_event(
            RuntimeOrigin::signed(1),
            0
        ));

        let pending = EventHub::get_pending_events();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending, vec![1, 2]);
    });
}

#[test]
fn event_statistics_tracked() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit events
        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::OnChain,
            vec![1],
            None
        ));

        assert_ok!(EventHub::submit_event(
            RuntimeOrigin::signed(1),
            EventType::CrossChain,
            vec![2],
            Some(2000)
        ));

        let stats = EventHub::get_statistics();
        assert_eq!(stats.total_events_submitted, 2);
        assert_eq!(stats.total_cross_chain_events, 1);
        assert_eq!(stats.total_events_processed, 0);

        // Process event
        assert_ok!(EventHub::process_event(
            RuntimeOrigin::signed(1),
            0
        ));

        let stats = EventHub::get_statistics();
        assert_eq!(stats.total_events_processed, 1);
    });
}

#[test]
fn on_initialize_processes_events() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit events
        for i in 0..3 {
            assert_ok!(EventHub::submit_event(
                RuntimeOrigin::signed(1),
                EventType::OnChain,
                vec![i],
                None
            ));
        }

        // Call on_initialize
        let _weight = EventHub::on_initialize(2);

        // Some events should be processed
        let stats = EventHub::get_statistics();
        assert!(stats.total_events_processed > 0);
    });
}
