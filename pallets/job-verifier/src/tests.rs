use crate::{mock::*, Error, Event, ProofType};
use frame_support::{assert_noop, assert_ok};
use pallet_job_registry::JobStatus;
use sp_core::H256;

#[test]
fn submit_proof_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit a job first
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

        // Submit proof
        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        // Check result was stored
        let result = JobVerifier::job_results(0).unwrap();
        assert_eq!(result.result_hash, result_hash);
        assert!(!result.verified);

        // Check event
        System::assert_has_event(Event::ProofSubmitted { job_id: 0, result_hash }.into());
    });
}

#[test]
fn submit_proof_for_nonexistent_job_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_noop!(
            JobVerifier::submit_proof(
                RuntimeOrigin::signed(1),
                999,
                result_hash,
                ProofType::Signature,
                proof_data
            ),
            Error::<Test>::JobNotFound
        );
    });
}

#[test]
fn submit_proof_for_pending_job_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit a job but don't start it
        assert_ok!(JobRegistry::submit_job(
            RuntimeOrigin::signed(1),
            vec![1, 2, 3],
            vec![],
            100
        ));

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_noop!(
            JobVerifier::submit_proof(
                RuntimeOrigin::signed(1),
                0,
                result_hash,
                ProofType::Signature,
                proof_data
            ),
            Error::<Test>::InvalidJobStatus
        );
    });
}

#[test]
fn verify_proof_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Setup: submit job and proof
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

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        // Verify proof
        assert_ok!(JobVerifier::verify_proof(
            RuntimeOrigin::signed(1),
            0
        ));

        // Check result is verified
        let result = JobVerifier::job_results(0).unwrap();
        assert!(result.verified);

        // Check event
        System::assert_has_event(Event::JobVerified { job_id: 0 }.into());
    });
}

#[test]
fn verify_already_verified_proof_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Setup and verify
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

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        assert_ok!(JobVerifier::verify_proof(
            RuntimeOrigin::signed(1),
            0
        ));

        // Try to verify again
        assert_noop!(
            JobVerifier::verify_proof(
                RuntimeOrigin::signed(1),
                0
            ),
            Error::<Test>::AlreadyVerified
        );
    });
}

#[test]
fn hash_proof_verification_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Setup job
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

        // Create proof data and its hash
        let proof_data = b"test result data".to_vec();
        let hash_bytes = sp_io::hashing::blake2_256(&proof_data);
        let result_hash = H256::from(hash_bytes);

        // Submit with hash proof type
        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Hash,
            proof_data
        ));

        // Verify - should succeed because hash matches
        assert_ok!(JobVerifier::verify_proof(
            RuntimeOrigin::signed(1),
            0
        ));

        let result = JobVerifier::job_results(0).unwrap();
        assert!(result.verified);
    });
}

#[test]
fn mark_verified_by_root_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Setup job and proof
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

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        // Mark as verified using root
        assert_ok!(JobVerifier::mark_verified(
            RuntimeOrigin::root(),
            0
        ));

        let result = JobVerifier::job_results(0).unwrap();
        assert!(result.verified);
    });
}

#[test]
fn verification_statistics_tracked() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Submit and verify one job
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

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        let stats = JobVerifier::get_stats();
        assert_eq!(stats.total_proofs_submitted, 1);
        assert_eq!(stats.total_proofs_verified, 0);

        assert_ok!(JobVerifier::verify_proof(
            RuntimeOrigin::signed(1),
            0
        ));

        let stats = JobVerifier::get_stats();
        assert_eq!(stats.total_proofs_verified, 1);
    });
}

#[test]
fn is_verified_helper_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        
        // Job doesn't exist
        assert!(!JobVerifier::is_verified(0));

        // Setup and verify job
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

        let result_hash = H256::from([1u8; 32]);
        let proof_data = vec![0u8; 64];

        assert_ok!(JobVerifier::submit_proof(
            RuntimeOrigin::signed(1),
            0,
            result_hash,
            ProofType::Signature,
            proof_data
        ));

        // Not yet verified
        assert!(!JobVerifier::is_verified(0));

        assert_ok!(JobVerifier::verify_proof(
            RuntimeOrigin::signed(1),
            0
        ));

        // Now verified
        assert!(JobVerifier::is_verified(0));
    });
}
