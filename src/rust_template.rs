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
    /// Program with a mint token template
    MintToken,
}

/// Create a program from the given name and template.
pub fn create_program(name: &str, template: ProgramTemplate) -> Result<()> {
    let program_path = Path::new("programs").join(name);
    let common_files = vec![
        ("Cargo.toml".into(), workspace_manifest().into()),
        (program_path.join("Cargo.toml"), cargo_toml(name, template)),
        (program_path.join("Xargo.toml"), xargo_toml().into()),
    ];

    let template_files = match template {
        ProgramTemplate::Basic => create_program_template_basic(name, &program_path),
        ProgramTemplate::Counter => create_program_template_counter(name, &program_path),
        ProgramTemplate::MintToken => create_program_template_mint_token(name, &program_path),
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

/// Create a program with mint token template
fn create_program_template_mint_token(name: &str, program_path: &Path) -> Files {
    vec![(
        program_path.join("src").join("lib.rs"),
        format!(
            r#"use anchor_lang::prelude::*;
use anchor_spl::{{
    associated_token::AssociatedToken,
    metadata::{{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    }},
    token::{{mint_to, Mint, MintTo, Token, TokenAccount}},
}};

declare_id!("{}");

#[program]
pub mod {} {{
    use super::*;
    pub fn init_token(ctx: Context<InitToken>, metadata: InitTokenParams) -> Result<()> {{
        // Define seeds and signer for creating a token account
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        // Define the token data with provided metadata
        let token_data: DataV2 = DataV2 {{
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        }};

        // Create context for the Metadata Accounts creation with the signer
        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {{
                payer: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            }},
            &signer,
        );

        // Call to create metadata accounts with the given token data
        create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;

        msg!("Token mint created successfully.");

        Ok(())
    }}

    pub fn mint_tokens(ctx: Context<MintTokens>, quantity: u64) -> Result<()> {{
        // Define seeds and signer for minting tokens
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        // Mint tokens to the destination account with the given quantity
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {{
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                }},
                &signer,
            ),
            quantity,
        )?;

        Ok(())
    }}
}}

// Struct defining the context for initializing a token
#[derive(Accounts)]
#[instruction(
    params: InitTokenParams
)]
pub struct InitToken<'info> {{
    /// CHECK: New Metaplex Account being created
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = params.decimals,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    /// CHECK: Metaplex program ID
    pub token_metadata_program: UncheckedAccount<'info>,
}}

// Struct defining the parameters for initializing a token
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitTokenParams {{
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
}}

// Struct defining the context for minting tokens
#[derive(Accounts)]
pub struct MintTokens<'info> {{
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init_if_needed, //Initializes the destination account if it does not exist
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub destination: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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

fn cargo_toml(name: &str, template: ProgramTemplate) -> String {
    let template_files = match template {
        ProgramTemplate::Basic => cargo_toml_basic(name),
        ProgramTemplate::Counter => cargo_toml_counter(name),
        ProgramTemplate::MintToken => cargo_toml_mint_token(name),
    };

    template_files
}

fn cargo_toml_basic(name: &str) -> String {
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

fn cargo_toml_counter(name: &str) -> String {
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

fn cargo_toml_mint_token(name: &str) -> String {
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
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]

[dependencies]
anchor-lang = {{ version = "{2}", features = ["init-if-needed"] }}
anchor-spl = {{ version = "{3}", features = ["metadata"] }}
"#,
        name,
        name.to_snake_case(),
        ANCHOR_VERSION,
        ANCHOR_VERSION
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

pub fn create_anchor_toml(
    program_id: String,
    test_script: String,
    template: ProgramTemplate,
) -> String {
    let template_files = match template {
        ProgramTemplate::Basic => create_anchor_toml_basic(program_id, test_script),
        ProgramTemplate::Counter => create_anchor_toml_counter(program_id, test_script),
        ProgramTemplate::MintToken => create_anchor_toml_mint_token(program_id, test_script),
    };

    template_files
}

pub fn create_anchor_toml_basic(program_id: String, test_script: String) -> String {
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

pub fn create_anchor_toml_counter(program_id: String, test_script: String) -> String {
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

pub fn create_anchor_toml_mint_token(program_id: String, test_script: String) -> String {
    format!(
        r#"[toolchain]

[features]
seeds = false
skip-lint = false

[programs.localnet]
counter = "{program_id}"
[programs.devnet]
counter = "{program_id}"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "devnet"
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
        ProgramTemplate::MintToken => ts_package_json_mint_token(license),
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
    "@coral-xyz/anchor": "^{ANCHOR_VERSION}"
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

pub fn ts_package_json_mint_token(license: String) -> String {
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
        ProgramTemplate::MintToken => ts_mocha_mint_token(name),
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

pub fn ts_mocha_mint_token(name: &str) -> String {
    format!(
        r#"import * as anchor from "@coral-xyz/anchor";
import {{ Program }} from "@coral-xyz/anchor";
import {{ PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY }} from "@solana/web3.js";
import {{ assert }} from "chai";
import BN from "bn.js";
import {{ {} }} from "../target/types/{}";

describe("{}", () => {{
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.{} as Program<{}>;

  // Metaplex Constants
  const METADATA_SEED = "metadata";
  const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
  );

  // Constants from our program
  const MINT_SEED = "mint";

  // Data for our tests
  const payer = provider.wallet.publicKey;
  const metadata = {{
    name: "Icy",
    symbol: "ICY",
    uri: "https://cdn.discordapp.com/emojis/1192768878183465062.png?size=240&quality=lossless",
    decimals: 9,
  }};
  const mintAmount = 10;

  // Derive the public key for our mint account
  const [mint] = PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_SEED)],
    program.programId
  );

  // Derive the public key for our metadata account using the Metaplex program
  const [metadataAddress] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(METADATA_SEED),
      TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    TOKEN_METADATA_PROGRAM_ID
  );

  it("initialize", async () => {{
    // Check if the mint account already exists
    const info = await provider.connection.getAccountInfo(mint);
    if (info) {{
      return; // Do not attempt to initialize if already initialized
    }}
    console.log("  Mint not found. Attempting to initialize.");

    // Define the accounts and arguments for the `initToken` function call
    const context = {{
      metadata: metadataAddress,
      mint,
      payer,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
    }};

    // Call the `initToken` function to initialize the mint account
    const txHash = await program.methods
      .initToken(metadata)
      .accounts(context)
      .rpc();

    // Wait for confirmation and log transaction details
    await provider.connection.confirmTransaction(txHash, "finalized");
    console.log(`  https://explorer.solana.com/tx/${{txHash}}?cluster=devnet`);

    // Verify that the mint account was initialized
    const newInfo = await provider.connection.getAccountInfo(mint);
    assert(newInfo, "  Mint should be initialized.");
  }});

  it("mint tokens", async () => {{
    // Derive the associated token account address for the payer
    const destination = anchor.utils.token.associatedAddress({{
      mint: mint,
      owner: payer,
    }});

    // Get initial token balance (0 if account not yet created)
    let initialBalance: number;
    try {{
      const balance = await provider.connection.getTokenAccountBalance(
        destination
      );
      initialBalance = balance.value.uiAmount;
    }} catch {{
      // Token account not yet initiated has 0 balance
      initialBalance = 0;
    }}

    // Define the accounts and arguments for the `mintTokens` function call
    const context = {{
      mint,
      destination,
      payer,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
    }};

    // Call the `mintTokens` function to mint tokens
    const txHash = await program.methods
      .mintTokens(new BN(mintAmount * 10 ** metadata.decimals))
      .accounts(context)
      .rpc();
    await provider.connection.confirmTransaction(txHash);
    console.log(`  https://explorer.solana.com/tx/${{txHash}}?cluster=devnet`);

    // check icy balance of payer
    const postBalance = (
      await provider.connection.getTokenAccountBalance(destination)
    ).value.uiAmount;
    assert.equal(
      initialBalance + mintAmount,
      postBalance,
      "Post balance should equal initial plus mint amount"
    );
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

pub fn readme(template: ProgramTemplate) -> String {
    let template_files = match template {
        ProgramTemplate::Basic => readme_basic(),
        ProgramTemplate::Counter => readme_counter(),
        ProgramTemplate::MintToken => readme_mint_token(),
    };

    template_files
}

pub fn readme_basic() -> String {
    r#"**Build Program**
```sh
anchor build
```

**Test Program**
```sh
anchor test 
```
"#
    .to_string()
}

pub fn readme_counter() -> String {
    r#"**Build Program**
```sh
anchor build
```

**Test Program**
```sh
anchor test 
```
"#
    .to_string()
}

pub fn readme_mint_token() -> String {
    r#"### How to Test for Creating Token and Minting Token to Other Wallet

Since the program utilizes the Metaplex program, deployment to the Devnet network is required.

1. **Configure Solana URL to Devnet**
    ```sh
    solana config set --url https://api.devnet.solana.com
    ```

2. **Build Program**
    ```sh
    anchor build
    ```

3. **Airdrop SOL to Address**
    - To deploy the `mint_token` program, ensure you have 2-3 SOL in the wallet which stores the `wallet.json` file.
    - Get the address of the wallet:
        ```shell
        solana address --keypair wallet.json
        ```
    - Airdrop to another address:
        ```shell
        solana airdrop 2 <address>
        ```
    - Check the balance of the address:
        ```shell
        solana balance <address>
        ```

4. **Deploy Program**
    ```sh
    anchor deploy
    ```

5. **Test Program**
    ```sh
    anchor test --skip-deploy
    ```
"#.to_string()
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
