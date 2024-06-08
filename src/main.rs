use anyhow::Result;
use clap::Parser;
use df_sol::Opts;

fn main() -> Result<()> {
    df_sol::entry(Opts::parse())
}
