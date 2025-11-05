# Keyword Impact Analyzer

A tool to analyze keyword usage across PHP packages to assess the impact of reserving new keywords in PHP.

Built for PHP internals developers and RFC authors to make data-driven decisions about language evolution.

## Quick Start

```bash
# Download and analyze top 10,000 packages
cargo run --release -- -k let -k using -k scope --max 10000

# Analyze existing downloads
cargo run --release -- -k with --skip-download
```

## Usage

```bash
keyword-impact-analyzer [OPTIONS] --keyword <KEYWORD>

Options:
  -k, --keyword <KEYWORD>  Keywords to analyze (repeatable)
  --min <MIN>              Minimum package index [default: 0]
  --max <MAX>              Maximum package index [default: 500]
  -d, --directory <DIR>    Download directory [default: downloads]
  --skip-download          Skip download phase
  -h, --help               Print help
```

## How It Works

1. **Download**: Fetches top N packages from Packagist
2. **Extract**: Extracts packages
3. **Analyze**: Parses PHP files and tracks keyword usage:
   - **Soft tracking**: Function names, function calls, and closure creations
   - **Hard tracking**: All identifiers (includes soft + symbol names, metohds, etc.)

## Output Format

Results are displayed as a table:

```
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| Keyword | Soft | Hard  | Soft Impact | Hard Impact | Well-Known Vendors                                    |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| with    |  174 | 36393 | High        | Critical    | doctrine, illuminate, laravel, phpunit, symfony, twig |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| scope   |    0 |  3428 | None        | Critical    | laravel, symfony                                      |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| block   |    1 |  2476 | Low         | Critical    | doctrine, laravel                                     |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| let     |    4 |  1031 | Low         | Critical    | -                                                     |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| using   |    0 |   485 | None        | High        | laravel, phpunit                                      |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| temp    |    0 |   197 | None        | High        | -                                                     |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
| scoped  |    2 |    54 | Low         | Medium      | -                                                     |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
```

**Impact Levels:**

- **None**: 0 occurrences
- **Low**: 1-25 occurrences
- **Medium**: 26-100 occurrences
- **High**: 101-500 occurrences
- **Extreme**: 501+

⚠️ **Warning**: Analysis of fewer than 200,000 files will show a warning recommending increasing `--max` for comprehensive results.

## Results

Analysis results for various RFCs:

- [Opt-in Block Scoping (`use` construct)](rfcs/optin_block_scoping.md) - 626,044 files analyzed
  - ✅ Safe: `scope`, `block`, `let`, `using`, `scoped`
  - ⚠️ Conflicts: `with` (174 occurrences)

## Contributing

To add analysis for a new RFC:

1. Run the tool with your candidate keywords
2. Create a new file in `rfcs/your_rfc_name.md`
3. Document the results and add a link in this README

## License

MIT
