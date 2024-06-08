use crate::rust_template::{create_anchor_toml, ProgramTemplate, TestTemplate};
use anyhow::{anyhow, Result};
use clap::Parser;
use heck::{ToKebabCase, ToSnakeCase};
use regex::Regex;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::string::ToString;

pub mod rust_template;
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Debug, Parser)]
#[clap(version = VERSION)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    Init {
        /// Workspace name
        name: String,
        /// Use JavaScript instead of TypeScript
        #[clap(short, long)]
        javascript: bool,
        /// Don't install JavaScript dependencies
        #[clap(long)]
        no_install: bool,
        /// Don't initialize git
        #[clap(long)]
        no_git: bool,
        /// Rust program template to use
        #[clap(value_enum, short, long, default_value = "single")]
        template: ProgramTemplate,
        /// Test template to use
        #[clap(value_enum, long, default_value = "mocha")]
        test_template: TestTemplate,
        /// Initialize even if there are files
        #[clap(long, action)]
        force: bool,
    },
}

pub fn entry(opts: Opts) -> Result<()> {
    let result = process_command(opts);

    result
}

fn process_command(opts: Opts) -> Result<()> {
    match opts.command {
        Command::Init {
            name,
            javascript,
            no_install,
            no_git,
            template,
            test_template,
            force,
        } => init(
            name,
            javascript,
            no_install,
            no_git,
            template,
            test_template,
            force,
        )
    }
}

fn set() -> Result<()> {
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn init(
    name: String,
    javascript: bool,
    no_install: bool,
    no_git: bool,
    template: ProgramTemplate,
    test_template: TestTemplate,
    force: bool,
) -> Result<()> {
    // We need to format different cases for the dir and the name
    let rust_name = name.to_snake_case();
    let project_name = if name == rust_name {
        rust_name.clone()
    } else {
        name.to_kebab_case()
    };

    // Additional keywords that have not been added to the `syn` crate as reserved words
    // https://github.com/dtolnay/syn/pull/1098
    let extra_keywords = ["async", "await", "try"];
    // Anchor converts to snake case before writing the program name
    if syn::parse_str::<syn::Ident>(&rust_name).is_err()
        || extra_keywords.contains(&rust_name.as_str())
    {
        return Err(anyhow!(
            "Anchor workspace name must be a valid Rust identifier. It may not be a Rust reserved word, start with a digit, or include certain disallowed characters. See https://doc.rust-lang.org/reference/identifiers.html for more detail.",
        ));
    }

    if force {
        fs::create_dir_all(&project_name)?;
    } else {
        fs::create_dir(&project_name)?;
    }
    std::env::set_current_dir(&project_name)?;
    fs::create_dir_all("app")?;

    let test_script = test_template.get_test_script(javascript);
    let program_id = rust_template::get_or_create_program_id(&rust_name);
    let toml = create_anchor_toml(program_id.to_string(), test_script.to_string());
    fs::write("Anchor.toml", toml)?;

    // Initialize .gitignore file
    fs::write(".gitignore", rust_template::git_ignore())?;

    // Initialize .prettierignore file
    fs::write(".prettierignore", rust_template::prettier_ignore())?;

    // Remove the default program if `--force` is passed
    if force {
        fs::remove_dir_all(
            std::env::current_dir()?
                .join("programs")
                .join(&project_name),
        )?;
    }

    // Build the program.
    rust_template::create_program(&project_name, template)?;

    // Build the migrations directory.
    fs::create_dir_all("migrations")?;

    let license = get_npm_init_license()?;

    let jest = TestTemplate::Jest == test_template;
    if javascript {
        // Build javascript config
        let mut package_json = File::create("package.json")?;
        package_json.write_all(rust_template::package_json(jest, license).as_bytes())?;

        let mut deploy = File::create("migrations/deploy.js")?;

        deploy.write_all(rust_template::deploy_script().as_bytes())?;
    } else {
        // Build typescript config
        let mut ts_config = File::create("tsconfig.json")?;
        ts_config.write_all(rust_template::ts_config(jest).as_bytes())?;

        let mut ts_package_json = File::create("package.json")?;
        ts_package_json.write_all(rust_template::ts_package_json(jest, license).as_bytes())?;

        let mut deploy = File::create("migrations/deploy.ts")?;
        deploy.write_all(rust_template::ts_deploy_script().as_bytes())?;
    }

    test_template.create_test_files(&project_name, javascript, &program_id.to_string())?;

    if !no_install {
        let yarn_result = install_node_modules("yarn")?;
        if !yarn_result.status.success() {
            println!("Failed yarn install will attempt to npm install");
            install_node_modules("npm")?;
        }
    }

    if !no_git {
        let git_result = std::process::Command::new("git")
            .arg("init")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("git init failed: {}", e.to_string()))?;
        if !git_result.status.success() {
            eprintln!("Failed to automatically initialize a new git repository");
        }
    }

    println!("{project_name} initialized");

    Ok(())
}

/// Array of (path, content) tuple.
pub type Files = Vec<(PathBuf, String)>;

/// Create files from the given (path, content) tuple array.
///
/// # Example
///
/// ```ignore
/// crate_files(vec![("programs/my_program/src/lib.rs".into(), "// Content".into())])?;
/// ```
pub fn create_files(files: &Files) -> Result<()> {
    for (path, content) in files {
        let path = Path::new(path);
        if path.exists() {
            continue;
        }

        match path.extension() {
            Some(_) => {
                fs::create_dir_all(path.parent().unwrap())?;
                fs::write(path, content)?;
            }
            None => fs::create_dir_all(path)?,
        }
    }

    Ok(())
}

/// Override or create files from the given (path, content) tuple array.
///
/// # Example
///
/// ```ignore
/// override_or_create_files(vec![("programs/my_program/src/lib.rs".into(), "// Content".into())])?;
/// ```
pub fn override_or_create_files(files: &Files) -> Result<()> {
    for (path, content) in files {
        let path = Path::new(path);
        if path.exists() {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)?;
            f.write_all(content.as_bytes())?;
            f.flush()?;
        } else {
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, content)?;
        }
    }

    Ok(())
}

fn install_node_modules(cmd: &str) -> Result<std::process::Output> {
    if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg(format!("/C {cmd} install"))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("{} install failed: {}", cmd, e.to_string()))
    } else {
        std::process::Command::new(cmd)
            .arg("install")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::format_err!("{} install failed: {}", cmd, e.to_string()))
    }
}

/// Get the system's default license - what 'npm init' would use.
fn get_npm_init_license() -> Result<String> {
    let npm_init_license_output = std::process::Command::new("npm")
        .arg("config")
        .arg("get")
        .arg("init-license")
        .output()?;

    if !npm_init_license_output.status.success() {
        return Err(anyhow!("Failed to get npm init license"));
    }

    let license = String::from_utf8(npm_init_license_output.stdout)?;
    Ok(license.trim().to_string())
}

fn get_anchor_version() -> Result<String> {
    let (cmd, args) = match cfg!(target_os = "windows") {
        true => ("cmd", vec!["/C", "anchor --version"]),
        false => ("sh", vec!["-c", "anchor --version"]),
    };

    let anchor_version_output = std::process::Command::new(cmd).args(args).output()?;

    if !anchor_version_output.status.success() {
        return Err(anyhow!("Failed to get anchor version"));
    }

    let anchor_version_string = String::from_utf8(anchor_version_output.stdout)?;

    // Define the regex to capture the version (assuming format "anchor 0.x.y")
    let re = Regex::new(r"(\d+\.\d+\.\d+)").unwrap();
    let cap = re.captures(&anchor_version_string);

    if let Some(mat) = cap {
        Ok(mat.get(1).unwrap().as_str().to_string())
    } else {
        Err(anyhow!("Failed to parse anchor version from output"))
    }
}
