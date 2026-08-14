//! Hub data store (stub — will use SQLite/PostgreSQL in production).

use pocker_core::plugin::PluginMetadata;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory plugin store (for development).
#[allow(dead_code)]
pub struct PluginStore {
    plugins: RwLock<HashMap<String, PluginMetadata>>,
}

#[allow(dead_code)]
impl PluginStore {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn publish(&self, metadata: PluginMetadata) {
        let mut plugins = self.plugins.write().unwrap();
        plugins.insert(metadata.name.clone(), metadata);
    }

    pub fn get(&self, name: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).cloned()
    }

    pub fn list(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().cloned().collect()
    }

    pub fn search(&self, query: &str) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        let query_lower = query.to_lowercase();
        plugins
            .values()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
}

impl Default for PluginStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocker_core::types::PluginType;

    #[test]
    fn store_publish_and_get() {
        let store = PluginStore::new();
        let mut meta = PluginMetadata::new("test-plugin", "1.0.0");
        meta.description = "A test plugin".to_string();
        meta.plugin_type = PluginType::Tool;

        store.publish(meta.clone());
        let got = store.get("test-plugin").unwrap();
        assert_eq!(got.name, "test-plugin");
        assert_eq!(got.version, "1.0.0");
    }

    #[test]
    fn store_search() {
        let store = PluginStore::new();
        let mut meta1 = PluginMetadata::new("code-review", "1.0.0");
        meta1.description = "Reviews code".to_string();
        let mut meta2 = PluginMetadata::new("data-analysis", "1.0.0");
        meta2.description = "Analyzes data".to_string();

        store.publish(meta1);
        store.publish(meta2);

        let results = store.search("code");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "code-review");
    }

    #[test]
    fn store_list() {
        let store = PluginStore::new();
        store.publish(PluginMetadata::new("p1", "1.0.0"));
        store.publish(PluginMetadata::new("p2", "2.0.0"));

        let list = store.list();
        assert_eq!(list.len(), 2);
    }
}
