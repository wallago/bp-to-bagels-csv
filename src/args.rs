use std::{fmt, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path of Bagels database
    #[arg(long,value_parser = db_path_exist)]
    pub db: String,

    /// Bagels account name
    #[arg(short, long, value_enum)]
    pub account: Account,

    /// Path of BP CSV file
    #[arg(long,value_parser = csv_path_exist)]
    pub csv: String,

    /// Parse and report without writing to the DB
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Account {
    CompteCourant,
    LivretA,
    Pel,
    LivretDD,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Account::CompteCourant => write!(f, "Compte Courant"),
            Account::LivretA => write!(f, "Livret A"),
            Account::Pel => write!(f, "PEL"),
            Account::LivretDD => write!(f, "LDD"),
        }
    }
}

fn db_path_exist(s: &str) -> Result<String> {
    let path = PathBuf::from_str(s)?;
    if !path.exists() {
        Err(anyhow::anyhow!("Bagels database path not exist"))
    } else {
        Ok(s.to_string())
    }
}

fn csv_path_exist(s: &str) -> Result<String> {
    let path = PathBuf::from_str(s)?;
    if !path.exists() {
        Err(anyhow::anyhow!("BP CSV path not exist"))
    } else {
        Ok(s.to_string())
    }
}
