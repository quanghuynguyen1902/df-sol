
# Df Sol

A Rust package designed to help you create template repositories easily to start with anchor framework on Solana. This README will guide you through the installation, usage, and contribution process for this package.

## Table of Contents

1. [Installation](#installation)
2. [Usage](#usage)
3. [Features](#features)

## Installation

To use package, you need to have Rust, solana-cli and anchor framework  installed on your system. If you don't have them installed:
- Install rust from [rust-lang.org](https://www.rust-lang.org/), 
- Install solana-cli from [here](https://docs.solanalabs.com/cli/install) and then run solana-keygen new to create a keypair at the default location. Anchor uses this keypair to run your program tests.
- Install anchor framework from [here](https://www.anchor-lang.com/docs/installation)

Once these packages are installed, install `df-sol` package by run:

```sh
cargo install df-sol
```

## Usage

### Creating a New Template

To create a new template repository, you can use the following command:

```sh
df-sol init <name-project>
``` 
Example
```sh
df-sol init counter
```

To create a program with multiple files for instructions, state...
```sh
df-sol init <name-project> --template multiple
``` 
Example
```sh
df-sol init counter --template multiple
```

To create a program with other test template
```sh
df-sol init <name-project> --test-template <test-template>
``` 
Test template includes: 
- mocha: Generate template for Mocha unit-test
- jest:  Generate template for Jest unit-test
- rust:  Generate template for Rust unit-test

Example
```sh
df-sol init counter --test-template jest
```

### Reporting Issues

If you encounter any issues, please report them on the [GitHub Issues](https://github.com/quanghuynguyen1902/df-sol/issues) page.

Thank you for using Df Sol! We hope this package helps you manage your templates efficiently. If you have any questions or feedback, feel free to open an issue on GitHub.
