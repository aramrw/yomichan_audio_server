mod tests {
    #[test]
    fn create_index() {
        crate::create_index_json(
            std::path::Path::new("F:/Programming/Rust/yomichan_http_server/audio/forvo_es"),
            "forvo_es",
            None,
            1,
        )
        .unwrap();
    }

    #[test]
    fn update_entries() {
        crate::update_entries();
    }
}
