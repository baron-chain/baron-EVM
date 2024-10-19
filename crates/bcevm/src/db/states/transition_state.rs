use super::TransitionAccount;
use bcevm_interpreter::primitives::{Address, HashMap};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct TransitionState {
    pub transitions: HashMap<Address, TransitionAccount>,
}

impl TransitionState {
    pub fn single(address: Address, transition: TransitionAccount) -> Self {
        let mut transitions = HashMap::new();
        transitions.insert(address, transition);
        Self { transitions }
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn add_transitions(&mut self, transitions: Vec<(Address, TransitionAccount)>) {
        for (address, account) in transitions {
            self.transitions
                .entry(address)
                .and_modify(|entry| entry.update(account.clone()))
                .or_insert(account);
        }
    }
}
