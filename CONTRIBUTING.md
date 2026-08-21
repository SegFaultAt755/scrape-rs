# Contributing

Thanks for your interest in the project! The rules are simple.

## Rules

1. **No spaghetti code.** Write clean, readable code: meaningful names, small functions, no copy-paste.
2. **Do not rewrite to async libraries.** This project is synchronous — keep it that way. Do not migrate code to tokio/async-std or similar without discussing it with the maintainer first.

## How to contribute

1. Fork the repo and create a branch from `main`.
2. Make your changes.
3. Make sure the project builds and tests pass:
   ```
   cargo build
   cargo test
   ```
4. Open a PR with a clear description of the changes.

## Code style

- Follow standard formatting: `cargo fmt`
- Check code with linter: `cargo clippy`
