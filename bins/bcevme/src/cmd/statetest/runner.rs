use bcevm::{db::EmptyDB, inspector_handle_register, inspectors::TracerEip3155, primitives::*, Evm, State};
use indicatif::{ProgressBar, ProgressDrawTarget};
use serde_json::json;
use std::{convert::Infallible, io::{stderr, stdout}, path::{Path, PathBuf}, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, Mutex}, time::Instant};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum TestErrorKind {
    #[error("logs root mismatch: got {got}, expected {expected}")]
    LogsRootMismatch { got: B256, expected: B256 },
    #[error("state root mismatch: got {got}, expected {expected}")]
    StateRootMismatch { got: B256, expected: B256 },
    #[error("unknown private key: {0:?}")]
    UnknownPrivateKey(B256),
    #[error("unexpected exception: got {got_exception:?}, expected {expected_exception:?}")]
    UnexpectedException { expected_exception: Option<String>, got_exception: Option<String> },
    #[error("unexpected output: got {got_output:?}, expected {expected_output:?}")]
    UnexpectedOutput { expected_output: Option<Bytes>, got_output: Option<Bytes> },
    #[error(transparent)]
    SerdeDeserialize(#[from] serde_json::Error),
    #[error("thread panicked")]
    Panic,
}

pub fn find_all_json_tests(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        WalkDir::new(path).into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension() == Some("json".as_ref()))
            .map(|e| e.into_path())
            .collect()
    }
}

fn check_evm_execution<EXT>(test: &Test, expected_output: Option<&Bytes>, test_name: &str, exec_result: &EVMResultGeneric<ExecutionResult, Infallible>, evm: &Evm<'_, EXT, &mut State<EmptyDB>>, print_json_outcome: bool) -> Result<(), TestError> {
    // Implementation details...
}

pub fn execute_test_suite(path: &Path, elapsed: &Arc<Mutex<Duration>>, trace: bool, print_json_outcome: bool) -> Result<(), TestError> {
    // Implementation details...
}

pub fn run(test_files: Vec<PathBuf>, mut single_thread: bool, trace: bool, mut print_outcome: bool, keep_going: bool) -> Result<(), TestError> {
    if trace { print_outcome = true; }
    if print_outcome { single_thread = true; }
    let n_files = test_files.len();

    let n_errors = Arc::new(AtomicUsize::new(0));
    let console_bar = Arc::new(ProgressBar::with_draw_target(Some(n_files as u64), ProgressDrawTarget::stdout()));
    let queue = Arc::new(Mutex::new((0usize, test_files)));
    let elapsed = Arc::new(Mutex::new(Duration::ZERO));

    let num_threads = if single_thread { 1 } else { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) }.min(n_files);
    let handles: Vec<_> = (0..num_threads).map(|i| {
        let (queue, n_errors, console_bar, elapsed) = (queue.clone(), n_errors.clone(), console_bar.clone(), elapsed.clone());
        std::thread::Builder::new().name(format!("runner-{i}")).spawn(move || {
            while !keep_going && n_errors.load(Ordering::SeqCst) == 0 {
                if let Some(test_path) = queue.lock().unwrap().1.get(queue.lock().unwrap().0).cloned() {
                    *queue.lock().unwrap().0 += 1;
                    console_bar.inc(1);
                    if let Err(err) = execute_test_suite(&test_path, &elapsed, trace, print_outcome) {
                        n_errors.fetch_add(1, Ordering::SeqCst);
                        if !keep_going { return Err(err); }
                    }
                } else { break; }
            }
            Ok(())
        }).unwrap()
    }).collect();

    let thread_errors: Vec<_> = handles.into_iter().enumerate()
        .filter_map(|(i, handle)| match handle.join() {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(_) => Some(TestError { name: format!("thread {i} panicked"), kind: TestErrorKind::Panic }),
        })
        .collect();

    console_bar.finish();
    println!("Finished execution. Total CPU time: {:.6}s", elapsed.lock().unwrap().as_secs_f64());

    let n_errors = n_errors.load(Ordering::SeqCst);
    if n_errors == 0 && thread_errors.is_empty() {
        println!("All tests passed!");
        Ok(())
    } else {
        println!("Encountered {n_errors} errors out of {n_files} total tests");
        if !thread_errors.is_empty() {
            println!("{} threads returned an error, out of {} total:", thread_errors.len(), num_threads);
            thread_errors.iter().for_each(|error| println!("{error}"));
        }
        Err(thread_errors.first().cloned().unwrap_or_else(|| TestError { name: "Unknown error".to_string(), kind: TestErrorKind::Panic }))
    }
}
