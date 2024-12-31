mod tests {
    #[test]
    fn create_index() {
        crate::create_index_json(
            std::path::Path::new("C:\\Users\\arami\\Desktop\\zh"),
            "forvo_zh",
            None,
            1,
        )
        .unwrap();
    }

    #[test]
    fn entries() {
        crate::update_entries();
    }
}
