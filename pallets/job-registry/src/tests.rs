use crate::{mock::*, Error, Event, JobStatus};
use frame_support::{assert_noop, assert_ok};

#[test]
fn submit_job_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        let metadata = vec![1, 2, 3, 4];
        let dependencies = vec![];
        let deadline = 100;

        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            metadata.clone(),
            dependencies,
            deadline
        ));

        // Check job was created
        let job = JobRegistry::jobs(0).unwrap();
        assert_eq!(job.owner, 1);
        assert_eq!(job.metadata.to_vec(), metadata);
        assert_eq!(job.deadline, 100);
        assert_eq!(job.status, JobStatus::Pending);

        // Check event
        System::assert_has_event(Event::JobSubmitted { job_id: 0, owner: 1 }.into());
    });
}

#[test]
fn submit_job_with_past_deadline_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        
        let metadata = vec![1, 2, 3, 4];
        let dependencies = vec![];
        let deadline = 5; // Past deadline

        assert_noop!(
            JobRegistry::submit_job(
                RuntimeOrigin::signed(1),
                metadata,
                dependencies,
                deadline
            ),
            Error::<Test>::DeadlineInPast
        );
    });
}

#[test]
fn submit_job_with_invalid_dependency_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        let metadata = vec![1, 2, 3, 4];
        let dependencies = vec![999]; // Non-existent job
        let deadline = 100;

        assert_noop!(
            JobRegistry::submit_job(
                RuntimeOrigin::signed(1),
                metadata,
                dependencies,
                deadline
            ),
            Error::<Test>::DependencyNotFound
        );
    });
}

#[test]
fn update_job_status_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        // Update to InProgress
        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::InProgress
        ));

        let job = JobRegistry::jobs(0).unwrap();
        assert_eq!(job.status, JobStatus::InProgress);

        // Check event
        System::assert_has_event(
            Event::JobStatusUpdated {
                job_id: 0,
                old_status: JobStatus::Pending,
                new_status: JobStatus::InProgress,
            }.into()
        );
    });
}

#[test]
fn update_job_status_unauthorized_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit job as user 1
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        // Try to update as user 2
        assert_noop!(
            JobRegistry::update_job_status(
                RuntimeOrigin::signed(2),
                0,
                JobStatus::InProgress
            ),
            Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn invalid_status_transition_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        // Try invalid transition: Pending -> Completed (must go through InProgress)
        assert_noop!(
            JobRegistry::update_job_status(
                RuntimeOrigin::signed(1),
                0,
                JobStatus::Completed
            ),
            Error::<Test>::InvalidStatusTransition
        );
    });
}

#[test]
fn job_with_dependencies_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit first job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1],
            vec![],
            100
        ));

        // Submit second job depending on first
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![2],
            vec![0],
            100
        ));

        let job = JobRegistry::jobs(1).unwrap();
        assert_eq!(job.dependencies.to_vec(), vec![0]);
    });
}

#[test]
fn remove_job_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit and complete job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::InProgress
        ));

        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::Completed
        ));

        // Remove job
        assert_ok!(JobRegistry::remove_job(
            RuntimeOrigin::signed(1),
            0
        ));

        // Check job is removed
        assert!(JobRegistry::jobs(0).is_none());

        // Check event
        System::assert_has_event(Event::JobRemoved { job_id: 0 }.into());
    });
}

#[test]
fn are_dependencies_met_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit first job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1],
            vec![],
            100
        ));

        // Submit second job depending on first
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![2],
            vec![0],
            100
        ));

        // Dependencies not met yet
        assert!(!JobRegistry::are_dependencies_met(1));

        // Complete first job
        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::InProgress
        ));
        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::Completed
        ));

        // Dependencies now met
        assert!(JobRegistry::are_dependencies_met(1));
    });
}

#[test]
fn get_ready_jobs_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit first job
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1],
            vec![],
            100
        ));

        // Submit second job depending on first
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![2],
            vec![0],
            100
        ));

        // Only first job is ready
        let ready = JobRegistry::get_ready_jobs();
        assert_eq!(ready, vec![0]);

        // Complete first job
        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::InProgress
        ));
        assert_ok!(JobRegistry::update_job_status(
            RuntimeOrigin::signed(1),
            0,
            JobStatus::Completed
        ));

        // Now second job is ready
        let ready = JobRegistry::get_ready_jobs();
        assert_eq!(ready, vec![1]);
    });
}
