use super::{Arc, FileState};
#[test]
fn shared_states_reuse_the_same_text_allocation() {
    let state = FileState::present("large contents".repeat(1_024));
    let FileState::Present(contents) = state else {
        panic!("constructed state must be present");
    };
    let shared = FileState::share(&contents);
    let FileState::Present(shared_contents) = shared else {
        panic!("shared state must be present");
    };
    assert!(Arc::ptr_eq(&contents.0, &shared_contents.0));
}
