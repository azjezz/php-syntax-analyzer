pub mod analyzer;
pub mod downloader;
pub mod extractor;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use analyzer::analyze_directory;

#[derive(Parser)]
#[command(name = "php-syntax-analyzer")]
#[command(version = "0.1.0")]
#[command(about = "Analyze PHP packages for keyword usage to assess impact of making keywords reserved", long_about = None)]
struct Cli {
    /// Target keyword to analyze (let, scope, or using)
    #[arg(
        short,
        long,
        help = "Target keyword to analyze (e.g, let, scope, or using)"
    )]
    keywords: Vec<String>,

    /// Minimum package index (0-based, inclusive)
    #[arg(long, default_value_t = 0)]
    min: usize,

    /// Maximum package index (0-based, exclusive)
    #[arg(long, default_value_t = 500)]
    max: usize,

    /// Directory to download packages to (analysis will run on sources subdirectory)
    #[arg(short, long, default_value = "downloads")]
    directory: PathBuf,

    /// Skip downloading packages (analyze existing sources only)
    #[arg(long, default_value_t = false)]
    skip_download: bool,

    /// Display found issues
    #[arg(long, default_value_t = false)]
    display: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::from_env("RUST_LOG")
                .add_directive(LevelFilter::INFO.into())
                .add_directive(
                    "mago_database::utils=error"
                        .parse()
                        .expect("Failed to parse RUST_LOG directive"),
                )
                .add_directive(
                    "hyper_util::client::legacy::pool=info"
                        .parse()
                        .expect("Failed to parse RUST_LOG directive"),
                )
                .add_directive(
                    "hyper_util::client::legacy::connect::http=info"
                        .parse()
                        .expect("Failed to parse RUST_LOG directive"),
                )
                .add_directive(
                    "reqwest::connect=info"
                        .parse()
                        .expect("Failed to parse RUST_LOG directive"),
                )
                .add_directive(
                    "hyper_util::client::legacy::connect::http=info"
                        .parse()
                        .expect("Failed to parse RUST_LOG directive"),
                ),
        )
        .with(
            fmt::layer()
                .without_time()
                .with_target(false)
                .with_thread_ids(false)
                .with_level(true),
        )
        .init();

    let cli = Cli::parse();

    if cli.keywords.is_empty() {
        anyhow::bail!("At least one keyword must be specified using --keywords");
    }

    if cli.min >= cli.max {
        anyhow::bail!("Minimum index must be less than maximum index");
    }

    let start_time = Instant::now();

    if !cli.skip_download {
        tracing::info!(
            "Downloading packages {} to {} to {:?}",
            cli.min,
            cli.max,
            cli.directory
        );

        let download_start = Instant::now();
        let successful = downloader::download_packages(cli.directory.clone(), cli.min, cli.max)
            .await
            .context("Failed to download packages")?;

        let download_duration = download_start.elapsed();
        tracing::info!(
            "Downloaded {} packages in {:.2}s",
            successful,
            download_duration.as_secs_f64()
        );
    } else {
        tracing::info!("Skipping download (--skip-download specified)");
    }

    let extract_start = Instant::now();
    let extracted =
        extractor::extract_packages(cli.directory.clone()).context("Failed to extract packages")?;

    let extract_duration = extract_start.elapsed();
    tracing::info!(
        "Extracted {} packages in {:.2}s",
        extracted,
        extract_duration.as_secs_f64()
    );

    let analysis_start = Instant::now();
    let sources_dir = cli.directory.join("sources");

    if !sources_dir.exists() {
        anyhow::bail!(
            "Sources directory does not exist: {:?}. Run without --skip-download first.",
            sources_dir
        );
    }

    analyze_directory(cli.directory, sources_dir, cli.keywords, cli.display)
        .context("Failed to analyze directory")?;

    let analysis_duration = analysis_start.elapsed();
    tracing::info!(
        "Analysis completed in {:.2}s",
        analysis_duration.as_secs_f64()
    );

    let total_duration = start_time.elapsed();
    tracing::info!("Total time: {:.2}s", total_duration.as_secs_f64());

    Ok(())
}
