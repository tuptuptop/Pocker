//! CLI command handlers.

use pocker_engine::Engine;
use anyhow::Result;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum PluginCommands {
    List,
    Info { name: String },
}

#[derive(Subcommand)]
pub enum ProfileCommands {
    List,
    Create { name: String, #[arg(long, default_value = "")] description: String },
    Switch { name: String },
}

#[derive(Subcommand)]
pub enum HubCommands {
    Search { query: String },
    Info { name: String },
}

#[derive(Subcommand)]
pub enum SystemCommands {
    Info,
}

pub fn handle_plugin(action: crate::PluginCommands, engine: &Engine) -> Result<()> {
    match action {
        crate::PluginCommands::List => {
            let plugins = engine.loader.list();
            if plugins.is_empty() {
                println!("No plugins registered.");
            } else {
                println!("{:<30} STATUS", "PLUGIN");
                println!("{:-<50}", "");
                for (name, mounted) in plugins {
                    let status = if mounted { "mounted" } else { "registered" };
                    println!("{:<30} {}", name, status);
                }
            }
        }
        crate::PluginCommands::Info { name } => {
            println!("Plugin: {}", name);
            // TODO: Show detailed plugin info
        }
    }
    Ok(())
}

pub fn handle_profile(action: crate::ProfileCommands, engine: &Engine) -> Result<()> {
    match action {
        crate::ProfileCommands::List => {
            let profiles = engine.list_profiles()?;
            if profiles.is_empty() {
                println!("No profiles found.");
            } else {
                println!("{:<20} STATUS", "PROFILE");
                println!("{:-<40}", "");
                let current = engine.current_profile();
                for name in profiles {
                    let status = if name == current { "* active" } else { "" };
                    println!("{:<20} {}", name, status);
                }
            }
        }
        crate::ProfileCommands::Create { name, description } => {
            engine.profiles.create(&name, &description, vec![])?;
            println!("Created profile: {}", name);
        }
        crate::ProfileCommands::Switch { name } => {
            engine.load_profile(&name)?;
            println!("Switched to profile: {}", name);
        }
    }
    Ok(())
}

pub async fn handle_run(
    name: &str,
    input: Option<String>,
    _engine: &Engine,
) -> Result<()> {
    println!("Running skill: {}", name);
    if let Some(input) = input {
        println!("Input: {}", input);
    }
    // TODO: Look up skill in ctx.skills and execute it
    println!("Skill execution not yet implemented.");
    Ok(())
}

pub fn handle_hub(action: crate::HubCommands) -> Result<()> {
    match action {
        crate::HubCommands::Search { query } => {
            println!("Searching Hub for: {}", query);
            // TODO: Connect to Pocker Hub and search
            println!("Hub search not yet implemented.");
        }
        crate::HubCommands::Info { name } => {
            println!("Hub info for: {}", name);
            // TODO: Fetch info from Hub
            println!("Hub info not yet implemented.");
        }
    }
    Ok(())
}

pub fn handle_system(action: crate::SystemCommands, engine: &Engine) -> Result<()> {
    match action {
        crate::SystemCommands::Info => {
            println!("Pocker System Information");
            println!("  Profile: {}", engine.current_profile());
            println!("  Plugins: {}", engine.loader.list().len());
            println!("  Seams:   {}", engine.context().list_seams().len());
        }
    }
    Ok(())
}
