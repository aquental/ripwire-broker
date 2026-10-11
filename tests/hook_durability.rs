//! Seam: what a hook forces to disk (PRD jev-mem §7 and §8.2, D-174). One test, alone in its
//! binary: it reads a counter of the whole process.
mod common;

use ripwire_broker::cli::{self, Command};
use ripwire_broker::memory::identity;
use ripwire_broker::memory::store::Store;
use ripwire_broker::state::{self, StateStore};
use serde_json::json;

/// A hook holds the agent at every edit, and forcing a file to disk costs it milliseconds each
/// time. What it writes is recreated or repaired by the next event, so it leaves the forcing to
/// the system; the server's writes and the store's own stay forced.
#[tokio::test]
async fn a_hook_forces_nothing_to_disk_and_what_a_power_loss_leaves_is_repaired() {
    if !common::ripwire_available() {
        eprintln!("skipping: ripwire not on PATH");
        return;
    }
    let (repo, st) = (common::sample_repo(), tempfile::tempdir().unwrap());
    let ws = repo.path().canonicalize().unwrap();
    common::write(
        &ws,
        "src/auth.py",
        "def login(user, token):\n    return 1\n",
    );
    let argv = [
        "hook",
        "claude-code",
        "post-tool-use",
        "--memory",
        "--state-dir",
        st.path().to_str().unwrap(),
    ];
    let Ok(Command::Hook(args)) = cli::parse(argv.map(String::from).to_vec()) else {
        panic!("the hook's arguments parse")
    };
    let event = json!({
        "session_id": "s1", "cwd": ws, "hook_event_name": "PostToolUse",
        "tool_name": "Edit", "tool_input": {"file_path": ws.join("src/auth.py")}
    })
    .to_string();

    let before = state::forced_syncs();
    ripwire_broker::hook::run(&args, &event).await;
    let store = Store::new(st.path(), &identity::workspace_id(&ws).unwrap());
    let sessions = StateStore::new(st.path().to_path_buf());
    assert_eq!(
        store.pending().unwrap(),
        1,
        "the observation is in the spool"
    );
    assert_eq!(sessions.sessions().len(), 1, "the session was saved");
    assert_eq!(
        state::forced_syncs() - before,
        0,
        "the hook waits for no disk: spool, session and status line"
    );

    // The same observation published by a process that is not a hook is still forced: its file
    // and the directory that names it.
    let spooled = std::fs::read_dir(store.dir().join("spool"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let record = ripwire_broker::memory::model::Record::parse(&std::fs::read(&spooled).unwrap())
        .expect("what the hook wrote is a whole record");
    let before = state::forced_syncs();
    store.enqueue(&record).unwrap();
    assert_eq!(state::forced_syncs() - before, 2, "file and directory");

    // What a power loss can leave of a write that was not forced: a file of the right name
    // with nothing in it. The spool drops it and goes on; the session starts fresh.
    std::fs::write(&spooled, b"").unwrap();
    std::fs::write(
        store.dir().join("spool").join(format!("{:064}.json", 7)),
        b"",
    )
    .unwrap();
    let ingested = store.ingest().unwrap();
    assert_eq!((ingested.added, ingested.rejected), (0, 2), "{ingested:?}");
    assert_eq!(store.pending().unwrap(), 0, "nothing is left to trip on");
    for entry in std::fs::read_dir(st.path()).unwrap().flatten() {
        if entry.path().extension().is_some_and(|x| x == "json") {
            std::fs::write(entry.path(), b"").unwrap();
        }
    }
    assert_eq!(
        sessions.load("s1"),
        ripwire_broker::hook::SessionState::default(),
        "an empty state file is a fresh session"
    );
    ripwire_broker::hook::run(&args, &event).await;
    assert_eq!(
        sessions.sessions().len(),
        1,
        "and the next hook saves it again"
    );
}
