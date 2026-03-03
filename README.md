# `tomt` - a TOML formatter

`tomt` is a dead simple TOML formatter that uses `taplo` under the hood.

It provides both a library and CLI that perform roughly the same functionality.

## Installation

```
# cargo install tomt
```

## Usage

`tomt` by default will attempt to find a `.tomlfmt.toml` file in the current and ancestor
directories. If found, it will run formatting starting in this directory.

Otherwise, without extra CLI parameters, it will run within the current directory.

When it runs, it will glob the filesystem recursively for all `.toml` files, ignoring any
that are `.gitignore`'d.

`tomt` exits non-zero only on I/O errors; TOML syntax errors are silently ignored and retained.
`tomt` is not a TOML linter, just a formatter.

Passing `-c`/`--check` causes `tomt` to exit non-zero if formatting _would_
cause a change in the file contents. This can be used for CI/CD steps, etc.

# License

Copyright &copy; 2026 by Josh Junon and released under the MIT _or_ the Apache-2.0 licenses, at your discretion.

Part of the [Oro Operating System](https://github.com/oro-os) project.
