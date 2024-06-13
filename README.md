
# Df Sol

A Rust package designed to help you create template repositories easily to start with anchor framework on Solana. This `README.md` file will guide you through the installation, usage, and contribution process for this package.

## Table of Contents

1. [Setting up the environment](#setting-up-the-environment)
2. [Creating a new project](#creating-a-new-project)
3. [Writing and compiling smart contracts](#writing-and-compiling-smart-contracts)
4. [Testing contracts](#testing-contracts)
5. [Deploying to a live network](#deploying-to-a-live-network)
6. [Integrate with Frontend](#integrate-with-frontend)

## Setting up the environment

To use package, you need to have Rust, nodejs, and git installed on your system. If you don't have them installed:
- Install `rust` from [rust-lang.org](https://www.rust-lang.org/), 
- Install `node.js >=18.0` from [nodejs.org](https://nodejs.org/en/download/package-manager)
- Install `git` from [git](https://www.atlassian.com/git/tutorials/install-git)

Once these packages are installed, install `df-sol` package by run:

```sh
cargo install df-sol
```

## Creating a new project

To create a new project, you can use the following command:

```sh
df-sol init <name-project>
``` 
Example
```sh
df-sol init counter
```

To create a project using an optional template
```sh
df-sol init <name-project> --template <template>
``` 
Example
```sh
df-sol init counter-program --template counter
```
Program template includes:
- **basic**: Generate basic template
- **counter**:  Generate counter template
- **mint-token**:  Generate mint token template

Navigate to the folder you created and use Devbox to install the environment.
If you don't install, follow Follow the instruction from [the installation guide](https://www.jetify.com/devbox/docs/installing_devbox/).
Open a terminal in that folder.
- Init devbox 
  ```shell
  devbox init
  ```
- Start a new shell
  ```shell
  devbox shell --pure
  ```

## Writing and compiling smart contracts

### Writing smart contracts
Start by creating a new directory called `programs` and write logic smart contract to file `./src/lib.rs`.

### Compiling contracts
To compile the contract run `anchor build` in your terminal
```shell
 anchor build
```

## Testing contracts
A file test is generated for you. The shell of the test imports the `Anchor framework` files and gets the program ready to run. The tests to using the `mocha` test framework, so each `it` function defines a test and describe can be used to group tests together.
```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Counter } from "../target/types/counter";

describe("counter", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Counter as Program<Counter>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
```

To run test, use:
```shell
anchor test
```

## Deploying to a live network

Once you're ready to share your dApp with other people, you may want to deploy it to a live network. This way others can access an instance that's not running locally on your system.

The `mainnet` Solana network deals with real money, but there are separate `devnet` networks that do not.

To deploy to the `devnet` network, follow these steps:

1. **Configure Solana URL to Devnet**
    ```shell
    solana config set --url https://api.devnet.solana.com
    ```

2. **Airdrop SOL to Address**
  - To deploy a program to the `devnet` network, ensure that you have SOL in your wallet. The amount of SOL depends on the size of the program.

  - Get the address of the wallet:
    ```shell
    solana address --keypair wallet.json
    ```

  - Airdrop SOL to your address:
    ```shell
    solana airdrop 2 <address>
    ```

  - Check the balance of the address:
    ```shell
    solana balance <address>
    ```

3. **Deploy Program**
    ```sh
    anchor deploy
    ```

4. **Test Program**
    ```sh
    anchor test --skip-deploy
    ```

### Integrate with Frontend
Import the generated TypeScript module into your front-end application, and use it to interact with your program. The module provides functions that correspond to the functions defined in your IDL.

Copy `idl` of your program from `target/idl/{project_name}.json` to file `idl.json` in your front-end project folder. Then, following the code below:

```typescript
import { Program, AnchorProvider, setProvider } from "@project-serum/anchor";
import { Connection, KeyPair, PublicKey } from "@solana/web3.js";
import idl from "./idl.json";
import { Idl } from "@coral-xyz/anchor";
// where IDL is the .json created by anchor build

export const yourFunction = async () => {
    const wallet = KeyPair.generate();
    const connection = new Connection("https://api.devnet.solana.com");
    const provider = new AnchorProvider(connection, wallet, {});
    setProvider(provider);
    const programId = new PublicKey(idl.address);
    const program = new Program(idl as Idl, programId);
    // ... your code
    // e.g. await program.methods.yourMethod(YOUR_PARAMETERS).accounts({YOUR_ACCOUNTS}).rpc();
};
```
  
## Reporting Issues

If you encounter any issues, please report them on the [GitHub Issues](https://github.com/quanghuynguyen1902/df-sol/issues) page.

Thank you for using Df Sol! We hope this package helps you manage your templates efficiently. If you have any questions or feedback, feel free to open an issue on GitHub.
