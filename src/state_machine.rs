/// 4-State Logic Gating State Machine
/// Real-Time Traversal Implementation

use crate::cache_coherency::CacheState;

/// State Transition Matrix for 4-State Logic
pub struct StateTransitionTable {
    /// Transition table: [current_state][event] -> next_state
    table: [[CacheState; 4]; 4],
}

/// Cache Events that trigger state transitions
#[repr(u8)]
pub enum CacheEvent {
    LocalRead = 0,
    LocalWrite = 1,
    RemoteRead = 2,
    RemoteWrite = 3,
}

impl StateTransitionTable {
    /// Initialize the 4-state MESI transition table
    pub const fn new() -> Self {
        use CacheState::*;
        use CacheEvent::*;

        // Pre-computed transition table for real-time performance
        let mut table = [[Invalid; 4]; 4];

        // Modified state transitions
        table[Modified as usize][LocalRead as usize] = Modified;
        table[Modified as usize][LocalWrite as usize] = Modified;
        table[Modified as usize][RemoteRead as usize] = Shared;
        table[Modified as usize][RemoteWrite as usize] = Invalid;

        // Exclusive state transitions
        table[Exclusive as usize][LocalRead as usize] = Exclusive;
        table[Exclusive as usize][LocalWrite as usize] = Modified;
        table[Exclusive as usize][RemoteRead as usize] = Shared;
        table[Exclusive as usize][RemoteWrite as usize] = Invalid;

        // Shared state transitions
        table[Shared as usize][LocalRead as usize] = Shared;
        table[Shared as usize][LocalWrite as usize] = Modified;
        table[Shared as usize][RemoteRead as usize] = Shared;
        table[Shared as usize][RemoteWrite as usize] = Invalid;

        // Invalid state transitions
        table[Invalid as usize][LocalRead as usize] = Shared;
        table[Invalid as usize][LocalWrite as usize] = Modified;
        table[Invalid as usize][RemoteRead as usize] = Invalid;
        table[Invalid as usize][RemoteWrite as usize] = Invalid;

        Self { table }
    }

    /// O(1) state transition lookup - real-time guarantee
    #[inline(always)]
    pub const fn transition(&self, current: CacheState, event: CacheEvent) -> CacheState {
        self.table[current as usize][event as usize]
    }
}

/// Real-Time Cache Coherency State Machine
pub struct CoherencyStateMachine {
    transition_table: StateTransitionTable,
}

impl CoherencyStateMachine {
    pub const fn new() -> Self {
        Self {
            transition_table: StateTransitionTable::new(),
        }
    }

    /// Execute state transition based on your 5-step flow
    #[inline]
    pub fn execute_flow(&self, step: u8, current_state: CacheState, is_local: bool) -> CacheState {
        match step {
            // Step 1: Core 1 reads data → Shared state
            1 => {
                if is_local {
                    self.transition_table.transition(current_state, CacheEvent::LocalRead)
                } else {
                    current_state
                }
            }
            // Step 2: Core 2 reads same data → also Shared
            2 => self.transition_table.transition(current_state, CacheEvent::RemoteRead),
            // Step 3: Core 1 writes → invalidates Core 2's copy
            3 => {
                if is_local {
                    self.transition_table.transition(current_state, CacheEvent::LocalWrite)
                } else {
                    // Remote core sees write from Core 1
                    self.transition_table.transition(current_state, CacheEvent::RemoteWrite)
                }
            }
            // Step 4: Core 2's cache line marked Invalid (handled in step 3)
            4 => CacheState::Invalid,
            // Step 5: Core 2 reads again → fetches from Core 1 or L3
            5 => self.transition_table.transition(CacheState::Invalid, CacheEvent::LocalRead),
            _ => current_state,
        }
    }

    /// Direct state transition
    #[inline(always)]
    pub fn transition(&self, current: CacheState, event: CacheEvent) -> CacheState {
        self.transition_table.transition(current, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coherency_flow() {
        let sm = CoherencyStateMachine::new();

        // Step 1: Core 1 reads → Invalid to Shared
        let state1 = sm.execute_flow(1, CacheState::Invalid, true);
        assert_eq!(state1, CacheState::Shared);

        // Step 2: Core 2 reads → remains Shared
        let state2 = sm.execute_flow(2, state1, false);
        assert_eq!(state2, CacheState::Shared);

        // Step 3: Core 1 writes → Shared to Modified (on Core 1)
        let state3_core1 = sm.execute_flow(3, state2, true);
        assert_eq!(state3_core1, CacheState::Modified);

        // Step 4: Core 2 sees write → Shared to Invalid
        let state4_core2 = sm.execute_flow(3, state2, false);
        assert_eq!(state4_core2, CacheState::Invalid);

        // Step 5: Core 2 reads again → Invalid to Shared
        let state5 = sm.execute_flow(5, state4_core2, true);
        assert_eq!(state5, CacheState::Shared);
    }
}
