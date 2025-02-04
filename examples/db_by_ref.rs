use bcevm::{
    db::{CacheDB, EmptyDB, WrapDatabaseRef},
    handler::register::HandleRegister,
    inspector_handle_register,
    inspectors::{NoOpInspector, TracerEip3155},
    primitives::ResultAndState,
    DatabaseCommit, DatabaseRef, Evm,
};
use std::error::Error;

/// A trait that ensures the database reference has a Debug error type.
/// This is useful for error handling and logging.
pub trait DatabaseRefDebugError: DatabaseRef<Error = Self::DBError> {
    type DBError: std::fmt::Debug + Error + Send + Sync + 'static;
}

/// Blanket implementation for any database that meets the requirements
impl<DBError, DB> DatabaseRefDebugError for DB
where
    DB: DatabaseRef<Error = DBError>,
    DBError: std::fmt::Debug + Error + Send + Sync + 'static,
{
    type DBError = DBError;
}

/// Run a transaction with a given database and external context
/// Returns both the transaction result and the modified database
pub fn run_transaction<EXT, DB: DatabaseRefDebugError>(
    db: DB,
    ext: EXT,
    register_handles_fn: HandleRegister<EXT, WrapDatabaseRef<DB>>,
) -> anyhow::Result<(ResultAndState, DB)> {
    let mut evm = Evm::builder()
        .with_ref_db(db)
        .with_external_context(ext)
        .append_handler_register(register_handles_fn)
        .build();

    let result = evm.transact()?;
    Ok((result, evm.into_context().evm.inner.db.0))
}

/// Run a transaction and commit changes to a database that supports commits
pub fn run_transaction_and_commit_with_ext<EXT, DB>(
    db: DB,
    ext: EXT,
    register_handles_fn: HandleRegister<EXT, WrapDatabaseRef<DB>>,
) -> anyhow::Result<()>
where
    DB: DatabaseRefDebugError + DatabaseCommit,
{
    let (ResultAndState { state: changes, .. }, mut db) = run_transaction(db, ext, register_handles_fn)?;
    db.commit(changes);
    Ok(())
}

/// Simplified version of run_transaction_and_commit specifically for CacheDB with EmptyDB
pub fn run_transaction_and_commit(db: &mut CacheDB<EmptyDB>) -> anyhow::Result<()> {
    let ResultAndState { state: changes, .. } = {
        let rdb = &*db;
        Evm::builder()
            .with_ref_db(rdb)
            .with_external_context(NoOpInspector)
            .append_handler_register(inspector_handle_register)
            .build()
            .transact()?
    };
    db.commit(changes);
    Ok(())
}

/// Example usage of the EVM functionality
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::stdout;

    #[test]
    fn test_basic_transaction() -> anyhow::Result<()> {
        let mut cache_db = CacheDB::new(EmptyDB::default());
        run_transaction_and_commit(&mut cache_db)?;
        Ok(())
    }

    #[test]
    fn test_transaction_with_tracer() -> anyhow::Result<()> {
        let mut cache_db = CacheDB::new(EmptyDB::default());
        let mut tracer = TracerEip3155::new(Box::new(stdout()));
        run_transaction_and_commit_with_ext(&mut cache_db, &mut tracer, inspector_handle_register)?;
        Ok(())
    }
}

/// Main function demonstrating usage of the EVM module
pub fn main() -> anyhow::Result<()> {
    // Initialize a new cache database with an empty backing store
    let mut cache_db = CacheDB::new(EmptyDB::default());

    // Create a tracer that outputs to stdout
    let mut tracer = TracerEip3155::new(Box::new(std::io::stdout()));

    // Run a transaction with tracing enabled
    run_transaction_and_commit_with_ext(&mut cache_db, &mut tracer, inspector_handle_register)?;

    // Run a simple transaction without tracing
    run_transaction_and_commit(&mut cache_db)?;

    Ok(())
}

// Additional helper types and functions can be added below as needed

/// Type alias for common transaction results
pub type TransactionResult = anyhow::Result<(ResultAndState, Box<dyn DatabaseCommit>)>;

/// Helper function to create a new EVM instance with default settings
pub fn create_default_evm<DB: DatabaseRef>(db: DB) -> Evm<NoOpInspector, WrapDatabaseRef<DB>> {
    Evm::builder()
        .with_ref_db(db)
        .with_external_context(NoOpInspector)
        .append_handler_register(inspector_handle_register)
        .build()
}
