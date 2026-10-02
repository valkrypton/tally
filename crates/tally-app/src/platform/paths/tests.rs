use super::*;

fn temp_paths(name: &str) -> Paths {
    let dir = std::env::temp_dir().join(format!("tally-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Paths::in_dir(dir)
}

#[test]
fn files_live_in_the_data_folder() {
    let paths = Paths::in_dir(PathBuf::from("/data/tally"));
    assert_eq!(paths.database, PathBuf::from("/data/tally/tally.db"));
    assert_eq!(paths.logs, PathBuf::from("/data/tally/logs"));
}

#[test]
fn only_one_instance_holds_the_lock() {
    let paths = temp_paths("lock");
    paths.create_data_dir().unwrap();
    let first = paths.lock_instance().unwrap();
    assert!(first.is_some());
    assert!(
        paths.lock_instance().unwrap().is_none(),
        "second copy must not start"
    );
    drop(first);
    assert!(
        paths.lock_instance().unwrap().is_some(),
        "free again after the first exits"
    );
    std::fs::remove_dir_all(&paths.data_dir).unwrap();
}
