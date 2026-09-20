use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// A lifecycle regression cannot strand the parent test runner on a broken join.
pub(crate) fn bounded(name: &str, test: impl FnOnce()) {
    if std::env::var("SILK_TEST_CHILD").as_deref() == Ok(name) {
        test();
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("SILK_TEST_CHILD", name)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "child test failed: {name}");
            return;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("test exceeded watchdog: {name}");
        }
        thread::sleep(Duration::from_millis(20));
    }
}
