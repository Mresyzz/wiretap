#[cfg(test)]
mod tests {
    use wiretap::model::{KeyValue, TrafficEntry};
    use wiretap::replay::generate_curl;
    use wiretap::store::TrafficStore;

    #[tokio::test]
    async fn test_traffic_store_ring_buffer_eviction() {
        let store = TrafficStore::new(3);

        for i in 1..=5 {
            let entry = TrafficEntry::new(
                "GET".to_string(),
                format!("/api/item/{}", i),
                None,
                format!("http://localhost:3000/api/item/{}", i),
                vec![],
                None,
                200,
                "OK".to_string(),
                vec![],
                None,
                10,
                false,
            );
            store.add(entry).await;
        }

        let list = store.list().await;
        assert_eq!(list.len(), 3);
        // 应该保留最后3条：3, 4, 5
        assert_eq!(list[0].path, "/api/item/3");
        assert_eq!(list[1].path, "/api/item/4");
        assert_eq!(list[2].path, "/api/item/5");
    }

    #[test]
    fn test_generate_curl() {
        let entry = TrafficEntry::new(
            "POST".to_string(),
            "/api/v1/login".to_string(),
            None,
            "http://api.service.internal/api/v1/login".to_string(),
            vec![
                KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                },
                KeyValue {
                    key: "Authorization".to_string(),
                    value: "Bearer token123".to_string(),
                },
                KeyValue {
                    key: "Host".to_string(),
                    value: "api.service.internal".to_string(),
                }, // Host 应该被过滤掉
            ],
            Some(r#"{"user":"admin"}"#.to_string()),
            200,
            "OK".to_string(),
            vec![],
            None,
            15,
            false,
        );

        let curl = generate_curl(&entry);
        assert!(curl.contains("curl -X POST"));
        assert!(curl.contains("-H \"Content-Type: application/json\""));
        assert!(curl.contains("-H \"Authorization: Bearer token123\""));
        assert!(!curl.contains("-H \"Host:"));
        assert!(curl.contains("-d \"{\\\"user\\\":\\\"admin\\\"}\""));
        assert!(curl.contains("\"http://api.service.internal/api/v1/login\""));
    }
}
