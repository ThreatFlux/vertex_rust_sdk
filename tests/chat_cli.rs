use assert_cmd::Command;
use std::env;
use std::sync::LazyLock;
use std::sync::Mutex;

static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn chat_command_exits_on_exit_input() {
    let _guard = ENV_LOCK.lock().unwrap();
    env::set_var("VERTEX_PROJECT_ID", "demo");
    env::set_var("VERTEX_REGION", "us-central1");
    env::set_var("GOOGLE_ACCESS_TOKEN", "token");

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("vertex").unwrap();
    cmd.arg("chat").arg("--model").arg("gemini-1.5-flash").write_stdin("exit\n");

    cmd.assert().success();
}
