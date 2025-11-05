# RFC: Opt-in Block Scoping (use construct)

**RFC Link**: https://wiki.php.net/rfc/optin_block_scoping

**Analysis Date**: November 2024

**Packages Analyzed**: 18,975 packages

**Files Analyzed**: 626,044 PHP files

## Command

```bash
./target/release/keyword-impact-analyzer -k let -k using -k scope -k block -k with -k scoped --skip-download
```

## Results

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
| scoped  |    2 |    54 | Low         | Medium      | -                                                     |
+---------+------+-------+-------------+-------------+-------------------------------------------------------+
```

## Conclusion

**Recommended Keywords**: `scope`, `block`, `let`, `using`, `scoped`

All all keywords have zero or negligible conflicts (≤5 occurrence) across over half a million PHP files from the most popular packages.

**Avoid**: `with`

The `with` keyword has 174 conflicts and would cause breaking changes in existing codebases if made soft reserved.

## Notes

- The RFC currently proposes the `use` keyword (which wasn't tested as it's already a reserved keyword in PHP)
