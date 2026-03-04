# gscfmt

A tool for formatting GSC code.

---

## Installation

`gscfmt` is distributed as a standalone executable.

1. Download the latest release.
2. Place `gscfmt.exe` somewhere on your system.
3. Add it to your system `PATH`.

## Usage

`gscfmt [OPTIONS] [FILE...]`

`gscfmt --stdin`

`gscfmt -w script.gsc`

### Options

- -w, --write           Format files in-place
- -c, --check           Exit with code 1 if formatting is needed (CI mode)
- -d, --diff            Print a unified diff
- --stdin               Read from stdin, write to stdout
- -h, --help            Show help
