use crate::{create_files, get_anchor_version, override_or_create_files, Files};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use dirs::home_dir;
use heck::{ToPascalCase, ToSnakeCase};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use std::fmt;
use std::str::FromStr;
use std::{
    fs::{self, File},
    io::Write as _,
    path::Path,
    process::Stdio,
};

/// Program initialization template
#[derive(Clone, Debug, Default, Eq, PartialEq, Parser, ValueEnum)]
pub enum ProgramTemplate {
    /// Program with a single `lib.rs` file
    #[default]
    Single,
    /// Program with multiple files for instructions, state...
    Multiple,
}

/// Create a program from the given name and template.
pub fn create_program(name: &str, template: ProgramTemplate) -> Result<()> {
    let program_path = Path::new("programs").join(name);
    let common_files = vec![
        ("Cargo.toml".into(), workspace_manifest().into()),
        (program_path.join("Cargo.toml"), cargo_toml(name)),
        (program_path.join("Xargo.toml"), xargo_toml().into()),
    ];

    let template_files = match template {
        ProgramTemplate::Single => create_program_template_single(name, &program_path),
        ProgramTemplate::Multiple => create_program_template_multiple(name, &program_path),
    };

    create_files(&[common_files, template_files].concat())
}

#[macro_export]
macro_rules! home_path {
    ($my_struct:ident, $path:literal) => {
        #[derive(Clone, Debug)]
        pub struct $my_struct(String);

        impl Default for $my_struct {
            fn default() -> Self {
                $my_struct(home_dir().unwrap().join($path).display().to_string())
            }
        }

        impl FromStr for $my_struct {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_owned()))
            }
        }

        impl fmt::Display for $my_struct {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

home_path!(WalletPath, ".config/solana/id.json");

/// Create a program with a single `lib.rs` file.
fn create_program_template_single(name: &str, program_path: &Path) -> Files {
    vec![(
        program_path.join("src").join("lib.rs"),
        format!(
            r#"use anchor_lang::prelude::*;

declare_id!("{}");

#[program]
pub mod {} {{
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {{
        Ok(())
    }}
}}

#[derive(Accounts)]
pub struct Initialize {{}}
"#,
            get_or_create_program_id(name),
            name.to_snake_case(),
        ),
    )]
}

/// Create a program with multiple files for instructions, state...
fn create_program_template_multiple(name: &str, program_path: &Path) -> Files {
    let src_path = program_path.join("src");
    vec![
        (
            src_path.join("lib.rs"),
            format!(
                r#"pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("{}");

#[program]
pub mod {} {{
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {{
        initialize::handler(ctx)
    }}
}}
"#,
                get_or_create_program_id(name),
                name.to_snake_case(),
            ),
        ),
        (
            src_path.join("constants.rs"),
            r#"use anchor_lang::prelude::*;

#[constant]
pub const SEED: &str = "anchor";
"#
            .into(),
        ),
        (
            src_path.join("error.rs"),
            r#"use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,
}
"#
            .into(),
        ),
        (
            src_path.join("instructions").join("mod.rs"),
            r#"pub mod initialize;

pub use initialize::*;
"#
            .into(),
        ),
        (
            src_path.join("instructions").join("initialize.rs"),
            r#"use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize {}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    Ok(())
}
"#
            .into(),
        ),
        (src_path.join("state").join("mod.rs"), r#""#.into()),
    ]
}

const fn workspace_manifest() -> &'static str {
    r#"[workspace]
members = [
    "programs/*"
]
resolver = "2"

[profile.release]
overflow-checks = true
lto = "fat"
codegen-units = 1
[profile.release.build-override]
opt-level = 3
incremental = false
codegen-units = 1
"#
}

fn cargo_toml(name: &str) -> String {
    let version = get_anchor_version().unwrap();
    format!(
        r#"[package]
name = "{0}"
version = "0.1.0"
description = "Created with Anchor"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "{1}"

[features]
default = []
cpi = ["no-entrypoint"]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
idl-build = ["anchor-lang/idl-build"]

[dependencies]
anchor-lang = "{2}"
"#,
        name,
        name.to_snake_case(),
        version,
    )
}

fn xargo_toml() -> &'static str {
    r#"[target.bpfel-unknown-unknown.dependencies.std]
features = []
"#
}

/// Read the program keypair file or create a new one if it doesn't exist.
pub fn get_or_create_program_id(name: &str) -> Pubkey {
    let keypair_path = Path::new("target")
        .join("deploy")
        .join(format!("{}-keypair.json", name.to_snake_case()));

    read_keypair_file(&keypair_path)
        .unwrap_or_else(|_| {
            let keypair = Keypair::new();
            write_keypair_file(&keypair, keypair_path).expect("Unable to create program keypair");
            keypair
        })
        .pubkey()
}

pub fn create_anchor_toml(program_id: String, test_script: String) -> String {
    let wallet_path = WalletPath::default().to_string();
    format!(
        r#"[toolchain]

[features]
seeds = false
skip-lint = false

[programs.localnet]
counter = "{program_id}"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet = "{wallet_path}"

[scripts]
test = "{test_script}"
"#,
    )
}

pub fn deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@coral-xyz/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
};
"#
}

pub fn ts_deploy_script() -> &'static str {
    r#"// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@coral-xyz/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.
};
"#
}

pub fn mocha(name: &str) -> String {
    format!(
        r#"const anchor = require("@coral-xyz/anchor");

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Is initialized!", async () => {{
    // Add your test here.
    const program = anchor.workspace.{};
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name,
        name.to_pascal_case(),
    )
}

pub fn jest(name: &str) -> String {
    format!(
        r#"const anchor = require("@coral-xyz/anchor");

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Is initialized!", async () => {{
    // Add your test here.
    const program = anchor.workspace.{};
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name,
        name.to_pascal_case(),
    )
}

pub fn package_json(jest: bool, license: String) -> String {
    let version = get_anchor_version().unwrap();
    if jest {
        format!(
            r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{version}"
  }},
  "devDependencies": {{
    "jest": "^29.0.3",
    "prettier": "^2.6.2"
  }}
}}
    "#
        )
    } else {
        format!(
            r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{version}"
  }},
  "devDependencies": {{
    "chai": "^4.3.4",
    "mocha": "^9.0.3",
    "prettier": "^2.6.2"
  }}
}}
"#
        )
    }
}

pub fn ts_package_json(jest: bool, license: String) -> String {
    let version = get_anchor_version().unwrap();
    if jest {
        format!(
            r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{version}"
  }},
  "devDependencies": {{
    "@types/bn.js": "^5.1.0",
    "@types/jest": "^29.0.3",
    "jest": "^29.0.3",
    "prettier": "^2.6.2",
    "ts-jest": "^29.0.2",
    "typescript": "^4.3.5"
  }}
}}
"#
        )
    } else {
        format!(
            r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{version}"
  }},
  "devDependencies": {{
    "chai": "^4.3.4",
    "mocha": "^9.0.3",
    "ts-mocha": "^10.0.0",
    "@types/bn.js": "^5.1.0",
    "@types/chai": "^4.3.0",
    "@types/mocha": "^9.0.0",
    "typescript": "^4.3.5",
    "prettier": "^2.6.2"
  }}
}}
"#
        )
    }
}

pub fn ts_mocha(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{ {} }} from "../target/types/{}";

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.{} as Program<{}>;

  it("Is initialized!", async () => {{
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name.to_pascal_case(),
        name.to_snake_case(),
        name,
        name.to_pascal_case(),
        name.to_pascal_case(),
    )
}

pub fn ts_jest(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{ {} }} from "../target/types/{}";

describe("{}", () => {{
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.{} as Program<{}>;

  it("Is initialized!", async () => {{
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  }});
}});
"#,
        name.to_pascal_case(),
        name.to_snake_case(),
        name,
        name.to_pascal_case(),
        name.to_pascal_case(),
    )
}

pub fn ts_config(jest: bool) -> &'static str {
    if jest {
        r#"{
  "compilerOptions": {
    "types": ["jest"],
    "typeRoots": ["./node_modules/@types"],
    "lib": ["es2015"],
    "module": "commonjs",
    "target": "es6",
    "esModuleInterop": true
  }
}
"#
    } else {
        r#"{
  "compilerOptions": {
    "types": ["mocha", "chai"],
    "typeRoots": ["./node_modules/@types"],
    "lib": ["es2015"],
    "module": "commonjs",
    "target": "es6",
    "esModuleInterop": true
  }
}
"#
    }
}

pub fn git_ignore() -> &'static str {
    r#".anchor
.DS_Store
target
**/*.rs.bk
node_modules
test-ledger
.yarn
"#
}

pub fn prettier_ignore() -> &'static str {
    r#".anchor
.DS_Store
target
node_modules
dist
build
test-ledger
"#
}

/// Test initialization template
#[derive(Clone, Debug, Default, Eq, PartialEq, Parser, ValueEnum)]
pub enum TestTemplate {
    /// Generate template for Mocha unit-test
    #[default]
    Mocha,
    /// Generate template for Jest unit-test
    Jest,
    /// Generate template for Rust unit-test
    Rust,
}

impl TestTemplate {
    pub fn get_test_script(&self, js: bool) -> &str {
        match &self {
            Self::Mocha => {
                if js {
                    "yarn run mocha -t 1000000 tests/"
                } else {
                    "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
                }
            }
            Self::Jest => {
                if js {
                    "yarn run jest"
                } else {
                    "yarn run jest --preset ts-jest"
                }
            }
            Self::Rust => "cargo test",
        }
    }

    pub fn create_test_files(&self, project_name: &str, js: bool, program_id: &str) -> Result<()> {
        match self {
            Self::Mocha => {
                // Build the test suite.
                fs::create_dir_all("tests")?;

                if js {
                    let mut test = File::create(format!("tests/{}.js", &project_name))?;
                    test.write_all(mocha(project_name).as_bytes())?;
                } else {
                    let mut mocha = File::create(format!("tests/{}.ts", &project_name))?;
                    mocha.write_all(ts_mocha(project_name).as_bytes())?;
                }
            }
            Self::Jest => {
                // Build the test suite.
                fs::create_dir_all("tests")?;

                let mut test = File::create(format!("tests/{}.test.js", &project_name))?;
                test.write_all(jest(project_name).as_bytes())?;
            }
            Self::Rust => {
                // Do not initilize git repo
                let exit = std::process::Command::new("cargo")
                    .arg("new")
                    .arg("--vcs")
                    .arg("none")
                    .arg("--lib")
                    .arg("tests")
                    .stderr(Stdio::inherit())
                    .output()
                    .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
                if !exit.status.success() {
                    eprintln!("'cargo new --lib tests' failed");
                    std::process::exit(exit.status.code().unwrap_or(1));
                }

                let mut files = Vec::new();
                let tests_path = Path::new("tests");
                files.extend(vec![(
                    tests_path.join("Cargo.toml"),
                    tests_cargo_toml(project_name),
                )]);
                files.extend(create_program_template_rust_test(
                    project_name,
                    tests_path,
                    program_id,
                ));
                override_or_create_files(&files)?;
            }
        }

        Ok(())
    }
}
pub fn tests_cargo_toml(name: &str) -> String {
    let version = get_anchor_version().unwrap();
    format!(
        r#"[package]
name = "tests"
version = "0.1.0"
description = "Created with Anchor"
edition = "2021"

[dependencies]
anchor-client = "{0}"
{1} = {{ version = "0.1.0", path = "../programs/{1}" }}
"#,
        version, name,
    )
}

/// Generate template for Rust unit-test
fn create_program_template_rust_test(name: &str, tests_path: &Path, program_id: &str) -> Files {
    let src_path = tests_path.join("src");
    vec![
        (
            src_path.join("lib.rs"),
            r#"#[cfg(test)]
mod test_initialize;
"#
            .into(),
        ),
        (
            src_path.join("test_initialize.rs"),
            format!(
                r#"use std::str::FromStr;

use anchor_client::{{
    solana_sdk::{{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
    }},
    Client, Cluster,
}};

#[test]
fn test_initialize() {{
    let program_id = "{0}";
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = read_keypair_file(&anchor_wallet).unwrap();

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id).unwrap();
    let program = client.program(program_id).unwrap();

    let tx = program
        .request()
        .accounts({1}::accounts::Initialize {{}})
        .args({1}::instruction::Initialize {{}})
        .send()
        .expect("");

    println!("Your transaction signature {{}}", tx);
}}
"#,
                program_id,
                name.to_snake_case(),
            ),
        ),
    ]
}
