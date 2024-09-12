use crate::{create_index_json, update_entries};

#[test]
fn create_index() {
    create_index_json(
        std::path::Path::new("C:\\Users\\arami\\Desktop\\zh"),
        "forvo_zh",
        None,
        1,
    )
    .unwrap();
}

#[test]
fn entries() {
    update_entries();
}
