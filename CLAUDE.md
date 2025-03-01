# Edinburgh Project Guide

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Test all: `cargo test`
- Test single: `cargo test test_name`
- Test with output: `cargo test -- --nocapture`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Build docs: `cargo doc`

## Code Style Guidelines

- **Naming**: Use snake_case for variables/functions, CamelCase for types/structs
- **Imports**: Group std imports first, then external crates, then local modules
- **Error Handling**: Use Result<T, E> with ? operator for propagating errors
- **Comments**: Document public APIs with /// rustdoc comments
- **Formatting**: Follow rustfmt conventions (4-space indentation)
- **Types**: Prefer strong typing with explicit type annotations on public APIs
- **Organization**: Keep modules focused on single responsibility
- **Testing**: Write unit tests in the same file as the code being tested
