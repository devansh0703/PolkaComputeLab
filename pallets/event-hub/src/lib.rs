#![cfg_attr(not(feature = "std"), no_std)]

//! # Event Hub Pallet
//!
//! This pallet manages event-based job triggering and supports cross-chain messaging via XCMP/XCM.
//! Events can trigger jobs locally or send messages to other parachains.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use pallet_job_registry::{JobStatus, Pallet as JobRegistry};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Event types
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum EventType {
        /// On-chain event (from this chain)
        OnChain,
        /// Cross-chain event (from XCM/XCMP)
        CrossChain,
        /// Timer-based event
        Timer,
        /// Condition-based event
        Condition,
    }

    /// Trigger action types
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum TriggerAction {
        /// Start a job
        StartJob(u64),
        /// Send XCM message
        SendXcmMessage,
        /// Execute custom logic
        Custom,
    }

    /// Event data structure
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct EventData<BlockNumber> {
        /// Event type
        pub event_type: EventType,
        /// Event payload (could be job params, XCM message, etc.)
        pub payload: BoundedVec<u8, ConstU32<512>>,
        /// Block number when event was created
        pub created_at: BlockNumber,
        /// Whether event has been processed
        pub processed: bool,
        /// Source parachain ID (for cross-chain events)
        pub source_para_id: Option<u32>,
    }

    /// Trigger rule structure
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct TriggerRule<AccountId, BlockNumber> {
        /// Rule owner
        pub owner: AccountId,
        /// Event ID to watch for
        pub event_id: u64,
        /// Action to take when triggered
        pub action: TriggerAction,
        /// Condition (if any)
        pub condition: Option<BoundedVec<u8, ConstU32<128>>>,
        /// Block number when rule was created
        pub created_at: BlockNumber,
        /// Whether rule is active
        pub active: bool,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_job_registry::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of events to store
        #[pallet::constant]
        type MaxEvents: Get<u32>;

        /// Maximum number of trigger rules per account
        #[pallet::constant]
        type MaxTriggersPerAccount: Get<u32>;
    }

    /// Counter for event IDs
    #[pallet::storage]
    #[pallet::getter(fn next_event_id)]
    pub type NextEventId<T> = StorageValue<_, u64, ValueQuery>;

    /// Counter for trigger rule IDs
    #[pallet::storage]
    #[pallet::getter(fn next_trigger_id)]
    pub type NextTriggerId<T> = StorageValue<_, u64, ValueQuery>;

    /// Map from EventId to EventData
    #[pallet::storage]
    #[pallet::getter(fn events)]
    pub type Events<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        EventData<BlockNumberFor<T>>,
    >;

    /// Map from TriggerId to TriggerRule
    #[pallet::storage]
    #[pallet::getter(fn triggers)]
    pub type Triggers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        TriggerRule<T::AccountId, BlockNumberFor<T>>,
    >;

    /// Map from AccountId to their trigger IDs
    #[pallet::storage]
    #[pallet::getter(fn account_triggers)]
    pub type AccountTriggers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u64, T::MaxTriggersPerAccount>,
        ValueQuery,
    >;

    /// Map from EventId to associated trigger IDs
    #[pallet::storage]
    #[pallet::getter(fn event_triggers)]
    pub type EventTriggers<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        BoundedVec<u64, ConstU32<100>>,
        ValueQuery,
    >;

    /// Pending events queue (for OCW processing)
    #[pallet::storage]
    #[pallet::getter(fn pending_events)]
    pub type PendingEvents<T: Config> = StorageValue<
        _,
        BoundedVec<u64, T::MaxEvents>,
        ValueQuery,
    >;

    /// Statistics
    #[pallet::storage]
    #[pallet::getter(fn event_stats)]
    pub type EventStatistics<T: Config> = StorageValue<
        _,
        EventStats,
        ValueQuery,
    >;

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct EventStats {
        pub total_events_submitted: u64,
        pub total_events_processed: u64,
        pub total_triggers_activated: u64,
        pub total_cross_chain_events: u64,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event submitted [event_id, event_type]
        EventSubmitted { event_id: u64, event_type: EventType },
        /// Trigger registered [trigger_id, event_id, owner]
        TriggerRegistered { trigger_id: u64, event_id: u64, owner: T::AccountId },
        /// Trigger activated [trigger_id, event_id]
        TriggerActivated { trigger_id: u64, event_id: u64 },
        /// Event processed [event_id]
        EventProcessed { event_id: u64 },
        /// Cross-chain event received [event_id, source_para_id]
        CrossChainEventReceived { event_id: u64, source_para_id: u32 },
        /// Trigger deactivated [trigger_id]
        TriggerDeactivated { trigger_id: u64 },
        /// Job triggered by event [job_id, event_id]
        JobTriggered { job_id: u64, event_id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Event not found
        EventNotFound,
        /// Trigger not found
        TriggerNotFound,
        /// Not authorized
        NotAuthorized,
        /// Event already processed
        AlreadyProcessed,
        /// Max triggers per account reached
        MaxTriggersReached,
        /// Payload too large
        PayloadTooLarge,
        /// Invalid action
        InvalidAction,
        /// Condition not met
        ConditionNotMet,
        /// Max events reached
        MaxEventsReached,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Process pending events
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Process some pending events each block
            let pending = PendingEvents::<T>::get();
            let mut processed = 0u32;
            
            for event_id in pending.iter().take(5) {
                if let Some(event) = Events::<T>::get(event_id) {
                    if !event.processed {
                        let _ = Self::process_event_internal(*event_id);
                        processed += 1;
                    }
                }
            }

            // Return weight based on processed events
            Weight::from_parts(10_000_000u64 * processed as u64, 0)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit an event
        ///
        /// # Parameters
        /// - `origin`: Event submitter
        /// - `event_type`: Type of event
        /// - `payload`: Event payload data
        /// - `source_para_id`: Source parachain ID (for cross-chain events)
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_event())]
        pub fn submit_event(
            origin: OriginFor<T>,
            event_type: EventType,
            payload: Vec<u8>,
            source_para_id: Option<u32>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            // Validate payload size
            let bounded_payload: BoundedVec<u8, ConstU32<512>> = payload
                .try_into()
                .map_err(|_| Error::<T>::PayloadTooLarge)?;

            // Generate event ID
            let event_id = NextEventId::<T>::get();
            NextEventId::<T>::put(event_id.saturating_add(1));

            // Create event
            let event_data = EventData {
                event_type: event_type.clone(),
                payload: bounded_payload,
                created_at: frame_system::Pallet::<T>::block_number(),
                processed: false,
                source_para_id,
            };

            Events::<T>::insert(event_id, event_data);

            // Add to pending queue
            PendingEvents::<T>::try_mutate(|pending| -> DispatchResult {
                pending.try_push(event_id).map_err(|_| Error::<T>::MaxEventsReached)?;
                Ok(())
            })?;

            // Update statistics
            EventStatistics::<T>::mutate(|stats| {
                stats.total_events_submitted = stats.total_events_submitted.saturating_add(1);
                if matches!(event_type, EventType::CrossChain) {
                    stats.total_cross_chain_events = stats.total_cross_chain_events.saturating_add(1);
                }
            });

            Self::deposit_event(Event::EventSubmitted { event_id, event_type });

            if let Some(para_id) = source_para_id {
                Self::deposit_event(Event::CrossChainEventReceived {
                    event_id,
                    source_para_id: para_id,
                });
            }

            Ok(())
        }

        /// Register a trigger rule
        ///
        /// # Parameters
        /// - `origin`: Rule owner
        /// - `event_id`: Event to watch
        /// - `action`: Action to take
        /// - `condition`: Optional condition
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::register_trigger())]
        pub fn register_trigger(
            origin: OriginFor<T>,
            event_id: u64,
            action: TriggerAction,
            condition: Option<Vec<u8>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate condition size
            let bounded_condition = if let Some(cond) = condition {
                Some(cond.try_into().map_err(|_| Error::<T>::PayloadTooLarge)?)
            } else {
                None
            };

            // Check max triggers
            let mut account_trigger_list = AccountTriggers::<T>::get(&who);
            ensure!(
                (account_trigger_list.len() as u32) < T::MaxTriggersPerAccount::get(),
                Error::<T>::MaxTriggersReached
            );

            // Generate trigger ID
            let trigger_id = NextTriggerId::<T>::get();
            NextTriggerId::<T>::put(trigger_id.saturating_add(1));

            // Create trigger rule
            let trigger = TriggerRule {
                owner: who.clone(),
                event_id,
                action,
                condition: bounded_condition,
                created_at: frame_system::Pallet::<T>::block_number(),
                active: true,
            };

            Triggers::<T>::insert(trigger_id, trigger);

            // Add to account triggers
            account_trigger_list.try_push(trigger_id)
                .map_err(|_| Error::<T>::MaxTriggersReached)?;
            AccountTriggers::<T>::insert(&who, account_trigger_list);

            // Add to event triggers
            EventTriggers::<T>::try_mutate(event_id, |triggers| -> DispatchResult {
                triggers.try_push(trigger_id).map_err(|_| Error::<T>::MaxTriggersReached)?;
                Ok(())
            })?;

            Self::deposit_event(Event::TriggerRegistered {
                trigger_id,
                event_id,
                owner: who,
            });

            Ok(())
        }

        /// Process an event and activate triggers
        ///
        /// # Parameters
        /// - `origin`: Anyone can process (typically OCW)
        /// - `event_id`: Event to process
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::process_event())]
        pub fn process_event(
            origin: OriginFor<T>,
            event_id: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            Self::process_event_internal(event_id)
        }

        /// Deactivate a trigger
        ///
        /// # Parameters
        /// - `origin`: Trigger owner
        /// - `trigger_id`: Trigger to deactivate
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::deactivate_trigger())]
        pub fn deactivate_trigger(
            origin: OriginFor<T>,
            trigger_id: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Triggers::<T>::try_mutate(trigger_id, |maybe_trigger| -> DispatchResult {
                let trigger = maybe_trigger.as_mut().ok_or(Error::<T>::TriggerNotFound)?;
                ensure!(trigger.owner == who, Error::<T>::NotAuthorized);
                
                trigger.active = false;

                Self::deposit_event(Event::TriggerDeactivated { trigger_id });
                Ok(())
            })
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        /// Internal event processing
        fn process_event_internal(event_id: u64) -> DispatchResult {
            let mut event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
            ensure!(!event.processed, Error::<T>::AlreadyProcessed);

            // Get triggers for this event
            let trigger_ids = EventTriggers::<T>::get(event_id);

            // Activate each trigger
            for trigger_id in trigger_ids.iter() {
                if let Some(trigger) = Triggers::<T>::get(trigger_id) {
                    if trigger.active {
                        let _ = Self::activate_trigger(*trigger_id, event_id, &trigger);
                    }
                }
            }

            // Mark event as processed
            event.processed = true;
            Events::<T>::insert(event_id, event);

            // Remove from pending queue
            PendingEvents::<T>::mutate(|pending| {
                pending.retain(|&id| id != event_id);
            });

            // Update statistics
            EventStatistics::<T>::mutate(|stats| {
                stats.total_events_processed = stats.total_events_processed.saturating_add(1);
            });

            Self::deposit_event(Event::EventProcessed { event_id });

            Ok(())
        }

        /// Activate a trigger
        fn activate_trigger(
            trigger_id: u64,
            event_id: u64,
            trigger: &TriggerRule<T::AccountId, BlockNumberFor<T>>,
        ) -> DispatchResult {
            // Check condition if present
            if trigger.condition.is_some() {
                // In a real implementation, evaluate the condition
                // For now, we'll assume conditions pass
            }

            // Execute action
            match &trigger.action {
                TriggerAction::StartJob(job_id) => {
                    // Update job status to InProgress
                    if let Some(_job) = JobRegistry::<T>::jobs(job_id) {
                        let _ = JobRegistry::<T>::update_job_status(
                            frame_system::RawOrigin::Signed(trigger.owner.clone()).into(),
                            *job_id,
                            JobStatus::InProgress,
                        );

                        Self::deposit_event(Event::JobTriggered {
                            job_id: *job_id,
                            event_id,
                        });
                    }
                }
                TriggerAction::SendXcmMessage => {
                    // XCM message sending would be implemented here
                    // For now, this is a placeholder
                }
                TriggerAction::Custom => {
                    // Custom logic would be implemented here
                }
            }

            // Update statistics
            EventStatistics::<T>::mutate(|stats| {
                stats.total_triggers_activated = stats.total_triggers_activated.saturating_add(1);
            });

            Self::deposit_event(Event::TriggerActivated { trigger_id, event_id });

            Ok(())
        }

        /// Get pending events
        pub fn get_pending_events() -> Vec<u64> {
            PendingEvents::<T>::get().to_vec()
        }

        /// Get statistics
        pub fn get_statistics() -> EventStats {
            EventStatistics::<T>::get()
        }

        /// Get triggers for an account
        pub fn get_account_triggers(account: &T::AccountId) -> Vec<u64> {
            AccountTriggers::<T>::get(account).to_vec()
        }
    }
}
