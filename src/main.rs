use anyhow::{Context, Result};
use args::Args;
use bp_to_bagels_csv::{
    models::bp::{Bp, BpRaw},
    utils::{
        add_operation, category_index, category_map, ensure_uncategorized, existing_record_counts,
        resolve_account,
    },
};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;
use rust_decimal::prelude::ToPrimitive;
use tracing::Level;

mod args;

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

    let account_id = resolve_account(&conn, &args.account.to_string())?;
    let uncategorized_id = ensure_uncategorized(&conn)?;
    let categories = category_index(&conn)?;

    // Count-based dedup: how many records already exist per (date, label, cents) for this
    // account. Each matching CSV row consumes one, so re-imports stay idempotent without
    // dropping genuinely distinct same-day operations.
    let mut remaining_counts = existing_record_counts(&conn, account_id)?;

    let mut inserted_operations = 0;
    let mut skipped_operations = 0;
    let mut lost_operations = 0;
    for row in &rows {
        pb.inc(1);

        let date_str = row
            .date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S%.6f")
            .to_string();
        // Match the f64-based key built from the DB in `existing_record_counts`.
        let cents = (row.amount.abs().to_f64().unwrap_or(0.0) * 100.0).round() as i64;
        let key = (date_str, row.label.clone(), cents);
        if let Some(remaining) = remaining_counts.get_mut(&key)
            && *remaining > 0
        {
            *remaining -= 1;
            tracing::warn!("skipping duplicate: {row:#?}");
            skipped_operations += 1;
            continue;
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
        anyhow::bail!("failed to disconnect properly from db due to {err:?}");
    }
    Ok(())
}
