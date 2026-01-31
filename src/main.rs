use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

use analyzer::analyze_directory;

mod analyzer;
mod downloader;
mod extractor;
mod files;
mod results;

#[derive(Parser)]
#[command(name = "keyword-impact-analyzer")]
#[command(version = "0.1.0")]
#[command(about = "Analyze keyword impact across PHP packages for RFC authors", long_about = None)]
struct Cli {
    /// Keywords to analyze (can be specified multiple times)
    #[arg(short, long, required = false)]
    keyword: Vec<String>,

    /// Labels to analyze ( goto label, and named arguments )
    #[arg(short, long, required = false)]
    label: Vec<String>,

    /// Minimum package index (0-based, inclusive)
    #[arg(long, default_value_t = 0)]
    min: usize,

    /// Maximum package index (0-based, exclusive)
    #[arg(long, default_value_t = 500)]
    max: usize,

    /// Download directory
    #[arg(short, long, default_value = "downloads")]
    directory: PathBuf,

    /// Skip download phase (analyze existing sources only)
    #[arg(long)]
    skip_download: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let indicatif_layer = IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(indicatif_layer)
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

    if cli.keyword.is_empty() && cli.label.is_empty() {
        anyhow::bail!("At least one keyword or label must be specified for analysis");
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
        let (successful, failed) =
            downloader::download_packages(cli.directory.clone(), cli.min, cli.max)
                .await
                .context("Failed to download packages")?;

        if failed > 0 {
            tracing::warn!(
                "Download complete: {} successful, {} failed",
                successful,
                failed
            );
        } else {
            tracing::info!("All {} packages downloaded successfully", successful);
        }

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

    let has_keywords = !cli.keyword.is_empty();
    let has_labels = !cli.label.is_empty();

    let report = analyze_directory(sources_dir, cli.keyword, cli.label)
        .context("Failed to analyze directory")?;

    let analysis_duration = analysis_start.elapsed();
    tracing::info!(
        "Analysis completed in {:.2}s",
        analysis_duration.as_secs_f64()
    );

    let total_duration = start_time.elapsed();
    tracing::info!("Total time: {:.2}s", total_duration.as_secs_f64());

    report.display_table(has_keywords, has_labels);

    Ok(())
}
