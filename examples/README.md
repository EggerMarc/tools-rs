# Toors Examples

This directory contains examples demonstrating how to use the Toors library.

## Running Examples

To run an example, use the following command from the root of the repository:

```
cargo run --example <example_name>
```

## Available Examples

### Basic Examples

- `basic`: A simple demonstration of registering and calling tools
  ```
  cargo run --example basic
  ```

## Example Structure

Each example directory typically contains:

- A `main.rs` file with the main example code
- Additional files if needed for the specific example
- A brief README explaining what the example demonstrates

## Creating New Examples

When creating a new example:

1. Create a new directory under `examples/`
2. Add a `main.rs` file with your example code
3. Update this README to include your example
4. If your example requires more than a single file, create appropriate modules

All examples should demonstrate good practices for using the Toors library.