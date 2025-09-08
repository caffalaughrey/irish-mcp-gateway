mod api;
mod cli;
mod clients;
mod core;
mod domain;
mod infra;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    infra::logging::init();

    // Check if we're running admin commands
    // TODO(refactor-fit-and-finish): Consider feature-gating CLI entry for smaller prod binary.
    if should_run_cli(std::env::args().len()) {
        let exit_code = cli::run().await;
        PROCESS_EXITER.exit(map_exit(exit_code));
    }
    infra::boot::run_server().await
}



#[inline]
fn should_run_cli(arg_len: usize) -> bool {
    arg_len > 1
}

pub trait Exiter {
    fn exit(&self, code: i32) -> !;
}

struct ProcessExiterType;
static PROCESS_EXITER: ProcessExiterType = ProcessExiterType;

impl Exiter for ProcessExiterType {
    fn exit(&self, code: i32) -> ! {
        std::process::exit(code)
    }
}

#[inline]
fn map_exit(code: std::process::ExitCode) -> i32 {
    match code {
        std::process::ExitCode::SUCCESS => 0,
        _ => 1,
    }
}

#[inline]
#[allow(dead_code)]
fn entry_with_exiter(args_len: usize, exiter: impl Exiter, code: std::process::ExitCode) {
    if should_run_cli(args_len) {
        exiter.exit(map_exit(code));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn core_module_compiles() {
        let _ = env!("CARGO_PKG_NAME");
    }

    struct TestExiter(std::sync::Arc<std::sync::Mutex<Option<i32>>>);

    impl TestExiter {
        fn new(store: std::sync::Arc<std::sync::Mutex<Option<i32>>>) -> Self {
            Self(store)
        }
    }

    impl Exiter for TestExiter {
        fn exit(&self, code: i32) -> ! {
            *self.0.lock().unwrap() = Some(code);
            panic!("test-exit");
        }
    }

    #[test]
    fn map_exit_maps_success_to_zero_and_else_to_one() {
        assert_eq!(map_exit(std::process::ExitCode::SUCCESS), 0);
        let nonzero = std::process::ExitCode::from(2);
        assert_eq!(map_exit(nonzero), 1);
    }

    #[test]
    fn entry_with_exiter_triggers_exit_when_args_present() {
        let store = std::sync::Arc::new(std::sync::Mutex::new(None));
        let exiter = TestExiter::new(store.clone());
        let result = std::panic::catch_unwind(|| {
            entry_with_exiter(2, exiter, std::process::ExitCode::SUCCESS);
        });
        assert!(result.is_err());
        assert_eq!(*store.lock().unwrap(), Some(0));
    }
}
