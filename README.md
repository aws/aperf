## Performance Data Assistant

A CLI tool to gather many pieces of performance data in one go.

## Requirements
* Rust toolchain (v1.61.0+) - https://www.rust-lang.org/tools/install

## Building from source
```
cargo build
cargo test
cargo run
```

## Usage
```
./performance-data-assistant -h
```

## Example
```
./performance-data-assistant -i 1 -p 10
```
* This collects the performance data in 1 second time intervals for 10 seconds.

## Logs
* env_logger is used to log information about the tool run to stdout.
* To see it, use `export PDA_LOG_LEVEL=info`.
* To see more detail, use `export PDA_LOG_LEVEL=debug`.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License.

