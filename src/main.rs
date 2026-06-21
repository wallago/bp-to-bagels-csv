use std::{fmt, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use bp_to_bagels_csv::{
    models::bp::{Bp, BpRaw},
    utils::{
        add_operation, add_transfer, category_index, category_map, ensure_uncategorized,
        is_duplicate, resolve_account,
    },
};
use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;
use tracing::Level;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of Bagels database
    #[arg(long,value_parser = db_path_exist)]
    db: String,

    /// Bagels account name
    #[arg(short, long, value_enum)]
    account: Account,

    /// Path of BP CSV file
    #[arg(long,value_parser = csv_path_exist)]
    csv: String,

    /// Parse and report without writing to the DB
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Account {
    CompteCourant,
    LivretA,
    Pel,
    LivretDevelopementDurable,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Account::CompteCourant => write!(f, "Compte Courant"),
            Account::LivretA => write!(f, "Livret A"),
            Account::Pel => write!(f, "PEL"),
            Account::LivretDevelopementDurable => write!(f, "Livret Developement Durable"),
        }
    }
}

fn transfer_dest(detail: &str) -> Option<Account> {
    let d = detail.to_uppercase();
    if d.contains("COMPTE DE DEPOT PARTIC") || d.contains("COMPTE DE CHEQUES") {
        Some(Account::CompteCourant)
    } else if d.contains("LIVRET DEVELOPPEMENT D") {
        Some(Account::LivretDevelopementDurable)
    } else if d.contains("LIVRET A") {
        Some(Account::LivretA)
    } else if d.contains("PEL") {
        Some(Account::Pel)
    } else {
        None
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

fn main() -> Result<()> {
    let args = Args::parse();
    let level = match args.verbose {
        0 => Level::WARN, // default: warnings + errors only
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE, // -vvv and beyond
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let conn = Connection::open(args.db)?;
    conn.pragma_update(None, "foreign_keys", true)?;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_path(args.csv)
        .context("cannot open BP CSV")?;

    let rows: Vec<Bp> = rdr
        .records()
        .map(|r| {
            let raw: BpRaw = r?.deserialize(None)?;
            Bp::try_from(raw)
        })
        .collect::<Result<Vec<Bp>>>()?;

    let pb = ProgressBar::new(rows.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40}] {pos}/{len} ({eta}) {msg}",
        )
        .unwrap(),
    );

    for account in Account::value_variants() {
        resolve_account(&conn, &account.to_string())?;
    }
    let account_id = resolve_account(&conn, &args.account.to_string())?;
    let uncategorized_id = ensure_uncategorized(&conn)?;
    let categories = category_index(&conn)?;

    let mut inserted_operations = 0;
    let mut skipped_operations = 0;
    let mut lost_operations = 0;
    for row in &rows {
        pb.inc(1);
        match is_duplicate(&conn, account_id, row) {
            Ok(true) => {
                tracing::warn!("skipping duplicate: {row:#?}");
                skipped_operations += 1;
                continue;
            }
            Ok(false) => {}
            Err(err) => {
                tracing::error!("duplicate check failed for {}: {err}", row.label);
                lost_operations += 1;
                continue;
            }
        }
        if args.dry_run {
            tracing::debug!(
                "{} {} {} {} {}",
                row.date,
                row.amount,
                row.label,
                row.category,
                row.subcategory
            );
            continue;
        }

        let is_internal = row.subcategory.eq_ignore_ascii_case("Virement interne");
        if is_internal && row.amount.is_sign_negative() {
            match transfer_dest(&row.detail) {
                Some(dest) => {
                    let to_id = resolve_account(&conn, &dest.to_string())?;
                    match add_transfer(&conn, account_id, to_id, row) {
                        Ok(_) => inserted_operations += 1,
                        Err(err) => {
                            lost_operations += 1;
                            tracing::error!("failing to add transfer {:?} due to {}", row, err);
                        }
                    }
                    continue;
                }
                None => tracing::warn!(
                    "internal transfer {:?}: destination unresolved, importing as normal record",
                    row.detail
                ),
            }
        }

        let category_id = categories
            .get(category_map(&row.category))
            .copied()
            .unwrap_or(uncategorized_id);
        match add_operation(&conn, account_id, category_id, row) {
            Ok(_) => inserted_operations += 1,
            Err(err) => {
                lost_operations += 1;
                tracing::error!("failing to add operation {:?} due to {}", row, err);
            }
        }
    }
    let summary = format!(
        "{inserted_operations} operation(s)\n=> Skipped {skipped_operations} operation(s)\n=> Lost {lost_operations} operation(s)"
    );

    if args.dry_run {
        tracing::info!("DRY RUN -- columns seen: {:?}", rdr.headers()?);
        pb.finish_with_message(format!("\n=> Would insert {summary}"));
    } else {
        pb.finish_with_message(format!("\n=> Inserted {summary}"));
    }
    if let Err(err) = conn.close() {
        anyhow::bail!("failed to disconnect proprely from db due to {err:?}");
    }
    Ok(())
}
