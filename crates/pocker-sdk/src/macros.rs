//! Helper macros for plugin development (stub — real macros would use syn/quote).

/// Declare a plugin with metadata.
/// Usage: `declare_plugin!("my-plugin", "1.0.0", "Description")`
#[macro_export]
macro_rules! declare_plugin {
    ($name:expr, $version:expr, $description:expr) => {
        fn plugin_metadata() -> $crate::PluginMetadata {
            let mut meta = $crate::PluginMetadata::new($name, $version);
            meta.description = $description.to_string();
            meta
        }
    };
}

/// Declare an LLM adapter plugin.
#[macro_export]
macro_rules! declare_llm_adapter {
    ($name:expr, $version:expr) => {
        fn plugin_metadata() -> $crate::PluginMetadata {
            let mut meta = $crate::PluginMetadata::new($name, $version);
            meta.plugin_type = $crate::pocker_core::types::PluginType::LlmAdapter;
            meta.provides = vec!["ctx.llm".to_string()];
            meta
        }
    };
}

/// Declare a skill plugin.
#[macro_export]
macro_rules! declare_skill {
    ($name:expr, $version:expr) => {
        fn plugin_metadata() -> $crate::PluginMetadata {
            let mut meta = $crate::PluginMetadata::new($name, $version);
            meta.plugin_type = $crate::pocker_core::types::PluginType::Skill;
            meta.provides = vec!["ctx.skills".to_string()];
            meta
        }
    };
}
