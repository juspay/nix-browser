use clap::Subcommand;
use clap_verbosity_flag::{InfoLevel, Verbosity};

pub mod ci;
mod completion;
pub mod health;
pub mod init;
pub mod show;

#[derive(Subcommand, Debug)]
pub enum Command {
    Show(show::ShowConfig),

    Init(init::InitConfig),

    CI(ci::CIConfig),

    Health(health::HealthConfig),

    /// Generates shell completion scripts
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl Command {
    pub async fn run(&self, verbosity: Verbosity<InfoLevel>) -> anyhow::Result<()> {
        match self {
            Command::Show(config) => config.run().await,
            Command::Init(config) => config.run().await,
            Command::CI(config) => config.run(verbosity).await,
            Command::Health(config) => config.run().await,
            Command::Completion { shell } => completion::generate_completion(*shell),
        }
    }
}
