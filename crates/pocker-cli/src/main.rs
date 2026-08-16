//! Pocker CLI — command-line interface.
//!
//! Usage:
//!   pocker web          — Start Web UI
//!   pocker tui          — Start TUI
//!   pocker plugin list  — List installed plugins
//!   pocker profile list — List profiles
//!   pocker run `<skill>`  — Run a skill
//!   pocker --dump-config — Dump plugin tree config

mod commands;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pocker",
    version,
    about = "Pocker — Plugin as a Service. Everything is a Plugin.",
    long_about = "Pocker is a cross-platform plugin runtime where everything (LLM, tools, skills, sandbox, UI) is a plugin."
)]
struct Cli {
    /// Profile to use
    #[arg(long, global = true)]
    profile: Option<String>,

    /// Dump the plugin tree config and exit
    #[arg(long, global = true)]
    dump_config: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Web UI (Pocker Studio)
    Web,
    /// Start the TUI
    Tui,
    /// Start headless API server
    Headless {
        #[arg(long, default_value = "3080")]
        port: u16,
    },
    /// Plugin management
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// Profile management
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
    },
    /// Run a skill directly
    Run {
        /// Skill name
        name: String,
        /// Input as JSON string
        #[arg(long)]
        input: Option<String>,
    },
    /// Hub operations
    Hub {
        #[command(subcommand)]
        action: HubCommands,
    },
    /// System information
    System {
        #[command(subcommand)]
        action: SystemCommands,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// List installed plugins
    List,
    /// Show plugin info
    Info { name: String },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List all profiles
    List,
    /// Create a new profile
    Create {
        name: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Switch to a profile
    Switch { name: String },
}

#[derive(Subcommand)]
enum HubCommands {
    /// Search for plugins
    Search { query: String },
    /// Show plugin info from Hub
    Info { name: String },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Show system info
    Info,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocker=info".into()),
        )
        .init();

    let cli = Cli::parse();

    // Create engine
    let engine = pocker_engine::Engine::new();

    // Load profile if specified
    if let Some(profile_name) = &cli.profile {
        if engine.profiles.exists(profile_name) {
            engine.load_profile(profile_name)?;
            println!("Loaded profile: {profile_name}");
        } else {
            eprintln!("Profile not found: {profile_name}");
        }
    } else if engine.profiles.exists("web") {
        engine.load_profile("web")?;
    } else {
        // Create default profile
        let _ = engine.profiles.create(
            "default",
            "Default profile",
            vec!["@pocker/core-bundle".to_string()],
        );
        engine.load_profile("default")?;
    }

    // Handle --dump-config
    if cli.dump_config {
        let dump = engine.dump();
        println!("{}", serde_yaml::to_string(&dump)?);
        return Ok(());
    }

    // Handle commands
    match cli.command {
        Some(Commands::Web) => {
            println!("Starting Pocker Studio (Web UI)...");
            println!("Visit: http://127.0.0.1:3080");
            // TODO: Start the web server (pocker-studio)
            println!("Web UI not yet implemented. Use --dump-config to inspect.");
        }
        Some(Commands::Tui) => {
            println!("Starting Pocker TUI...");
            // TODO: Start the TUI (pocker-tui)
            println!("TUI not yet implemented.");
        }
        Some(Commands::Headless { port }) => {
            println!("Starting headless API on port {port}...");
            // TODO: Start the API server
            println!("Headless API not yet implemented.");
        }
        Some(Commands::Plugin { action }) => {
            commands::handle_plugin(action, &engine)?;
        }
        Some(Commands::Profile { action }) => {
            commands::handle_profile(action, &engine)?;
        }
        Some(Commands::Run { name, input }) => {
            commands::handle_run(&name, input, &engine)?;
        }
        Some(Commands::Hub { action }) => {
            commands::handle_hub(action)?;
        }
        Some(Commands::System { action }) => {
            commands::handle_system(&action, &engine)?;
        }
        None => {
            // No command — show help
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}
