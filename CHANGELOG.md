# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project _somewhat_ adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2023-07-31

- Fix windows double input (thanks [@programingjd](https://github.com/programingjd)!)
- Re-added a saved changed warning when backspacing/deleting bytes

## [0.4.0] - 2023-07-25

### Added

- This CHANGELOG file. This may be retroactively modified solely include changes earlier versions.
- You can now switch the endianness displayed on the labels with `control e` (thanks, [@JungleTryne](https://github.com/JungleTryne))!
- You can now open big files (thanks, [@Programatic](https://github.com/Programatic))! Note that this isn't a fully completed and perfected feature, and some things may still run slowly. Please [submit a bug report](https://github.com/ndd7xv/heh/issues) if you notice an issue.
- There are now templates for submitting issues; not required but there in case someone finds it useful

### Changed

- Search allows for iterating through all matched patterns with `control n` and `control p` (thanks, [@joooooooooooooooooooooooooooooooooooosh](https://github.com/joooooooooooooooooooooooooooooooooooosh)!)
- `heh` switched its dependency from `tui` to `ratatui`

### Other Notes

- MSRV bumped from 1.64.0 to 1.65.0
- memmap2 and crossbeam added as dependencies
- There was an error with CI that was fixed
- This release is 1191568 bytes on my machine, up from 1121936 in 0.3.0