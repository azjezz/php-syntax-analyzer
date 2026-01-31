# PHP RFC: Partial Function Application for instance of non-static methods ("$this")

**RFC Link**: https://wiki.php.net/rfc/partial_function_application_this

**Analysis Date**: January 2025

**Packages Analyzed**: 18,975 packages

**Files Analyzed**: 626,044 PHP files

## Command

```bash
./target/release/keyword-impact-analyzer -l this --skip-download
```

## Results

```
 INFO Skipping download (--skip-download specified)
⠲ extracting-packages{target_dir="downloads"}
 INFO extracting-packages{target_dir="downloads"}: Extracting 18988 packages...
⠤ extracting-packages{target_dir="downloads"}
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package debril/rss-atom-bundle from "downloads/zipballs/debril/rss-atom-bundle/debril-rss-atom-bundle.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package itsjavi/bootstrap-colorpicker from "downloads/zipballs/itsjavi/bootstrap-colorpicker/itsjavi-bootstrap-colorpicker.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package php-extended/php-http-message-psr7 from "downloads/zipballs/php-extended/php-http-message-psr7/php-extended-php-http-message-psr7.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package php-extended/php-charset-object from "downloads/zipballs/php-extended/php-charset-object/php-extended-php-charset-object.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package php-extended/php-charset-interface from "downloads/zipballs/php-extended/php-charset-interface/php-extended-php-charset-interface.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package socialknowledge/vue-env from "downloads/zipballs/socialknowledge/vue-env/socialknowledge-vue-env.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package brandembassy/datetime-factory from "downloads/zipballs/brandembassy/datetime-factory/brandembassy-datetime-factory.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package brandembassy/datetime from "downloads/zipballs/brandembassy/datetime/brandembassy-datetime.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package brandembassy/mockery-tools from "downloads/zipballs/brandembassy/mockery-tools/brandembassy-mockery-tools.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package t3/min from "downloads/zipballs/t3/min/t3-min.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package jigal/t3adminer from "downloads/zipballs/jigal/t3adminer/jigal-t3adminer.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package paypal/sdk-core-php from "downloads/zipballs/paypal/sdk-core-php/paypal-sdk-core-php.zip"
 WARN extracting-packages{target_dir="downloads"}: Failed to extract package studyportals/template4 from "downloads/zipballs/studyportals/template4/studyportals-template4.zip"
 WARN extracting-packages{target_dir="downloads"}: Extraction complete: 18975 successful, 13 failed
 INFO Extracted 18975 packages in 4.09s
 INFO analyzing-directory{sources_directory="downloads/sources" keywords=[] labels=["this"]}: Starting analysis...
⠂ analyzing-directory{sources_directory="downloads/sources" keywords=[] labels=["this"]}
 INFO analyzing-directory{sources_directory="downloads/sources" keywords=[] labels=["this"]}: Collected matches from 626044 files.
 INFO analyzing-directory{sources_directory="downloads/sources" keywords=[] labels=["this"]}: Analysis complete.
 INFO Analysis completed in 170.04s
 INFO Total time: 173.32s
 INFO No labels match found in the analyzed packages.
```

## Conclusion

The label `this` is safe to be used as there is 0 instances of it being used as a named argument or a goto label across over half a million PHP files from the most popular packages.
