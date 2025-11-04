# PHP Syntax Analyzer

Analyzes PHP packages to assess the impact of making keywords reserved in PHP.

## Usage

```bash
# Download and analyze packages
cargo run --release -- --keyword let --min 0 --max 100

# Analyze existing downloads
cargo run --release -- --keyword scope --skip-download

# Run full analysis
./analyze.sh
```

## Options

- `-k, --keyword <KEYWORD>` - Target keyword: let, scope, or using
- `--min <MIN>` - Minimum package index (default: 0)
- `--max <MAX>` - Maximum package index (default: 50)
- `-d, --directory <DIR>` - Download directory (default: ./downloads)
- `--skip-download` - Analyze existing sources only

## Output

Results are saved in `results/` directory with one file per keyword analysis.
