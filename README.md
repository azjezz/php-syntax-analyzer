# PHP Syntax Analyzer

Analyzes PHP packages to assess the impact of making keywords reserved in PHP.

Created for RFC: https://wiki.php.net/rfc/optin_block_scoping

## Usage

```bash
# Download packages
cargo run --release -- -k let --max 10000

# Analyze multiple keywords
cargo run --release -- -k let -k using -k scope -k block -k with --skip-download
```

## Options

- `-k, --keyword <KEYWORD>` - Target keyword (repeatable for multiple keywords)
- `--min <MIN>` - Minimum package index (default: 100)
- `--max <MAX>` - Maximum package index (default: 500)
- `-d, --directory <DIR>` - Download directory (default: ./downloads)
- `--skip-download` - Analyze existing sources only
- `--display` - Display found issues

## Results

Analysis of **507,529 PHP files** from 14,372 packages (top 10,000 most popular):

```
./target/release/php-syntax-analyzer -k let -k using -k scope -k block -k with --skip-download
 INFO Skipping download (--skip-download specified)
 INFO Extracting 14381 packages...
 WARN Failed to extract package: Failed to extract package debril/rss-atom-bundle from "downloads/zipballs/debril/rss-atom-bundle/debril-rss-atom-bundle.zip"
 WARN Failed to extract package: Failed to extract package php-extended/php-charset-object from "downloads/zipballs/php-extended/php-charset-object/php-extended-php-charset-object.zip"
 WARN Failed to extract package: Failed to extract package php-extended/php-charset-interface from "downloads/zipballs/php-extended/php-charset-interface/php-extended-php-charset-interface.zip"
 WARN Failed to extract package: Failed to extract package socialknowledge/vue-env from "downloads/zipballs/socialknowledge/vue-env/socialknowledge-vue-env.zip"
 WARN Failed to extract package: Failed to extract package brandembassy/datetime-factory from "downloads/zipballs/brandembassy/datetime-factory/brandembassy-datetime-factory.zip"
 WARN Failed to extract package: Failed to extract package brandembassy/datetime from "downloads/zipballs/brandembassy/datetime/brandembassy-datetime.zip"
 WARN Failed to extract package: Failed to extract package brandembassy/mockery-tools from "downloads/zipballs/brandembassy/mockery-tools/brandembassy-mockery-tools.zip"
 WARN Failed to extract package: Failed to extract package t3/min from "downloads/zipballs/t3/min/t3-min.zip"
 WARN Failed to extract package: Failed to extract package studyportals/template4 from "downloads/zipballs/studyportals/template4/studyportals-template4.zip"
 WARN Extraction complete: 14372 successful, 9 failed
 INFO Extracted 14372 packages in 2.26s
 INFO Starting analysis for keywords '["let", "using", "scope", "block", "with"]' in directory "downloads"
 INFO Analyzing 507529 files for 5 keywords...
 INFO No issues found for keyword 'let'.
 INFO No issues found for keyword 'using'.
 INFO No issues found for keyword 'scope'.
ERROR Analysis complete for keyword 'block'. Found 1 issues.
ERROR Analysis complete for keyword 'with'. Found 111 issues.
 INFO Analysis completed in 37.36s
 INFO Total time: 39.61s
```

| Keyword | Issues Found | Status       |
| ------- | ------------ | ------------ |
| let     | 0            | ✅ Safe      |
| using   | 0            | ✅ Safe      |
| scope   | 0            | ✅ Safe      |
| block   | **1**        | ✅ Safe      |
| with    | **111**      | ⚠️ Conflicts |

**Conclusion**: `with` has 111 conflicts and would break existing code if made a reserved keyword. Keywords `let`, `using`, and `scope` have zero conflicts. `block` has only 1 occurrence across half a million files.
