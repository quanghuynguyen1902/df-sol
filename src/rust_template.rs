use crate::{create_files, Files};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use heck::{ToPascalCase, ToSnakeCase};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use std::fs::File;
use std::io::Write;
use std::{fs, path::Path};

const ANCHOR_VERSION: &str = "0.30.0";

/// Program initialization template
#[derive(Clone, Debug, Default, Eq, PartialEq, Parser, ValueEnum, Copy)]
pub enum ProgramTemplate {
    /// Program with a basic template
    #[default]
    Basic,
    /// Program with a counter template
    Counter,
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
        ProgramTemplate::Basic => create_program_template_basic(name, &program_path),
        ProgramTemplate::Counter => create_program_template_counter(name, &program_path),
    };

    create_files(&[common_files, template_files].concat())
}

/// Create a program with a basic template
fn create_program_template_basic(name: &str, program_path: &Path) -> Files {
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

/// Create a program with counter template
fn create_program_template_counter(name: &str, program_path: &Path) -> Files {
    vec![(
        program_path.join("src").join("lib.rs"),
        format!(
            r#"use anchor_lang::prelude::*;

declare_id!("{}");

#[program]
pub mod {} {{
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {{
        let counter_account = &mut ctx.accounts.counter;
        counter_account.count = 0;
        Ok(())
    }}

    pub fn increment(ctx: Context<Increment>) -> Result<()> {{
        let counter_account = &mut ctx.accounts.counter;
        counter_account.count += 1;
        Ok(())
    }}
}}

#[derive(Accounts)]
pub struct Initialize<'info> {{
    #[account(
        init,
        seeds = [b"counter"],
        bump,
        payer=user,
        space = Counter::space()
    )]
    pub counter: Account<'info, Counter>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>
}}

#[derive(Accounts)]
pub struct Increment<'info> {{
    #[account(mut)]
    pub counter: Account<'info, Counter>,

    #[account(mut)]  // Remove leading space
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>
}}

#[account]
pub struct Counter {{
    count: u64
}}

impl Counter {{
    pub fn space() -> usize {{
        8 +  // discriminator
        8 // counter
    }}
}}
"#,
            get_or_create_program_id(name),
            name.to_snake_case(),
        ),
    )]
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
        ANCHOR_VERSION,
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
wallet = "wallet.json"

[scripts]
test = "{test_script}"
"#,
    )
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

pub fn ts_package_json(license: String, template: ProgramTemplate) -> String {
    let template_files = match template {
        ProgramTemplate::Basic => ts_package_json_basic(license),
        ProgramTemplate::Counter => ts_package_json_counter(license),
    };

    template_files
}

pub fn ts_package_json_basic(license: String) -> String {
    format!(
        r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{ANCHOR_VERSION}",
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

pub fn ts_package_json_counter(license: String) -> String {
    format!(
        r#"{{
  "license": "{license}",
  "scripts": {{
    "lint:fix": "prettier */*.js \"*/**/*{{.js,.ts}}\" -w",
    "lint": "prettier */*.js \"*/**/*{{.js,.ts}}\" --check"
  }},
  "dependencies": {{
    "@coral-xyz/anchor": "^{ANCHOR_VERSION}",
    "@solana/web3.js": "^1.92.3"
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

pub fn ts_mocha(name: &str, template: ProgramTemplate) -> String {
    let template_files = match template {
        ProgramTemplate::Basic => ts_mocha_basic(name),
        ProgramTemplate::Counter => ts_mocha_counter(name),
    };

    template_files
}

pub fn ts_mocha_basic(name: &str) -> String {
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

pub fn ts_mocha_counter(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{  PublicKey }} from "@solana/web3.js";
import {{ expect }} from "chai";
import {{ {} }} from "../target/types/{}";


describe("{}", () => {{
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.{} as Program<{}>;
  let counterAccount: PublicKey;
  let counterBump: number;

  before("Boilerplates", async () => {{
    [counterAccount, counterBump] = await PublicKey.findProgramAddress(
      [Buffer.from("counter")],
      program.programId
    );
  }});

  it("Initialize counter!", async () => {{
    await program.methods
      .initialize()
      .accounts({{
        counter: counterAccount,
        user: provider.wallet.publicKey,
      }})
      .rpc();

    const counter = await program.account.counter.fetch(counterAccount);
    expect(counter.count.toString()).eq("0")
  }});
  it("Increment counter", async () => {{
    await program.methods
      .increment()
      .accounts({{
        counter: counterAccount,
        user: provider.wallet.publicKey,
      }})
      .rpc();

    const counter = await program.account.counter.fetch(counterAccount);
    expect(counter.count.toString()).eq("1")
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

pub fn ts_config() -> &'static str {
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

pub fn git_ignore() -> &'static str {
    r#".anchor
.DS_Store
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

pub fn get_test_script() -> &'static str {
    "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
}

pub fn create_test_files(project_name: &str, template: ProgramTemplate) -> Result<()> {
    fs::create_dir_all("tests")?;

    let mut mocha = File::create(format!("tests/{}.ts", &project_name))?;
    mocha.write_all(ts_mocha(project_name, template).as_bytes())?;

    Ok(())
}

pub fn devbox_json() -> String {
    format!(
        r#"{{
  "packages": {{
    "curl": {{
      "version": "latest"
    }},
    "nodejs": {{
      "version": "18"
    }},
    "yarn": {{
      "version": "latest"
    }},
    "libiconv": {{
      "version": "latest"
    }},
    "darwin.apple_sdk.frameworks.Security": {{
      "platforms": [
        "aarch64-darwin",
        "x86_64-darwin"
      ]
    }},
    "darwin.apple_sdk.frameworks.SystemConfiguration": {{
      "platforms": [
        "aarch64-darwin",
        "x86_64-darwin"
      ]
    }}
  }},
  "shell": {{
    "init_hook": [
      "curl \"https://sh.rustup.rs\" -sfo rustup.sh && sh rustup.sh -y && rustup component add rustfmt clippy",
      "export PATH=\"${{HOME}}/.cargo/bin:${{PATH}}\"",
      "sh -c \"$(curl -sSfL https://release.solana.com/v1.18.16/install)\"",
      "export PATH=\"$HOME/.local/share/solana/install/active_release/bin:$PATH\"",
      "cargo install --git https://github.com/coral-xyz/anchor avm --locked --force",
      "avm install {ANCHOR_VERSION}",
      "avm use latest",
      "cargo install df-sol"
    ]
  }}
}}"#
    )
}
