use super::{cache::CacheState, state::DBBox, BundleState, State, TransitionState};
use crate::db::EmptyDB;
use bcevm_interpreter::primitives::{
    db::{Database, DatabaseRef, WrapDatabaseRef},
    B256,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateBuilder<DB> {
    database: DB,
    with_state_clear: bool,
    with_bundle_prestate: Option<BundleState>,
    with_cache_prestate: Option<CacheState>,
    with_bundle_update: bool,
    with_background_transition_merge: bool,
    with_block_hashes: BTreeMap<u64, B256>,
}

impl StateBuilder<EmptyDB> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<DB: Database + Default> Default for StateBuilder<DB> {
    fn default() -> Self {
        Self::new_with_database(DB::default())
    }
}

impl<DB: Database> StateBuilder<DB> {
    pub fn new_with_database(database: DB) -> Self {
        Self {
            database,
            with_state_clear: true,
            with_cache_prestate: None,
            with_bundle_prestate: None,
            with_bundle_update: false,
            with_background_transition_merge: false,
            with_block_hashes: BTreeMap::new(),
        }
    }

    pub fn with_database<ODB: Database>(self, database: ODB) -> StateBuilder<ODB> {
        StateBuilder {
            database,
            with_state_clear: self.with_state_clear,
            with_cache_prestate: self.with_cache_prestate,
            with_bundle_prestate: self.with_bundle_prestate,
            with_bundle_update: self.with_bundle_update,
            with_background_transition_merge: self.with_background_transition_merge,
            with_block_hashes: self.with_block_hashes,
        }
    }

    pub fn with_database_ref<ODB: DatabaseRef>(self, database: ODB) -> StateBuilder<WrapDatabaseRef<ODB>> {
        self.with_database(WrapDatabaseRef(database))
    }

    pub fn with_database_boxed<Error>(self, database: DBBox<'_, Error>) -> StateBuilder<DBBox<'_, Error>> {
        self.with_database(database)
    }

    pub fn without_state_clear(mut self) -> Self {
        self.with_state_clear = false;
        self
    }

    pub fn with_bundle_prestate(mut self, bundle: BundleState) -> Self {
        self.with_bundle_prestate = Some(bundle);
        self
    }

    pub fn with_bundle_update(mut self) -> Self {
        self.with_bundle_update = true;
        self
    }

    pub fn with_cached_prestate(mut self, cache: CacheState) -> Self {
        self.with_cache_prestate = Some(cache);
        self
    }

    pub fn with_background_transition_merge(mut self) -> Self {
        self.with_background_transition_merge = true;
        self
    }

    pub fn with_block_hashes(mut self, block_hashes: BTreeMap<u64, B256>) -> Self {
        self.with_block_hashes = block_hashes;
        self
    }

    pub fn build(self) -> State<DB> {
        let use_preloaded_bundle = self.with_cache_prestate.is_none() && self.with_bundle_prestate.is_some();
        State {
            cache: self.with_cache_prestate.unwrap_or_else(|| CacheState::new(self.with_state_clear)),
            database: self.database,
            transition_state: self.with_bundle_update.then(TransitionState::default),
            bundle_state: self.with_bundle_prestate.unwrap_or_default(),
            use_preloaded_bundle,
            block_hashes: self.with_block_hashes,
        }
    }
}
