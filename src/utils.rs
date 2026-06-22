use std::collections::HashMap;

use anyhow::Result;
use chrono::Local;
use rusqlite::{Connection, OptionalExtension, params};
use rust_decimal::prelude::ToPrimitive;

use crate::models::bp::Bp;

pub fn resolve_account(conn: &Connection, name: &str) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM account WHERE name = ?1 AND deletedAt IS NULL",
            params![name],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    conn.execute(
        "INSERT INTO account (createdAt, updatedAt, name, description, beginningBalance, hidden) \
         VALUES (?1, ?2, ?3, '', 0, 0)",
        params![now, now, name],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Count, per account, how many records already exist for each
/// `(date, label, amount-in-cents)` triple. Used for count-based dedup so re-running an
/// import is idempotent without dropping genuinely distinct same-day operations.
pub fn existing_record_counts(
    conn: &Connection,
    account_id: i64,
) -> Result<HashMap<(String, String, i64), i64>> {
    let mut stmt = conn.prepare(
        "SELECT date, label, amount, COUNT(*) FROM record \
         WHERE accountId = ?1 GROUP BY date, label, amount",
    )?;

    let rows = stmt.query_map(params![account_id], |row| {
        let date: String = row.get(0)?;
        let label: String = row.get(1)?;
        let amount: f64 = row.get(2)?;
        let count: i64 = row.get(3)?;
        Ok(((date, label, (amount * 100.0).round() as i64), count))
    })?;

    let map = rows.collect::<rusqlite::Result<HashMap<(String, String, i64), i64>>>()?;
    Ok(map)
}

pub fn category_index(conn: &Connection) -> Result<HashMap<String, i64>> {
    let mut stmt = conn.prepare("SELECT id, name FROM category WHERE deletedAt IS NULL")?;

    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        Ok((name, id))
    })?;

    let map = rows.collect::<rusqlite::Result<HashMap<String, i64>>>()?;
    Ok(map)
}

pub fn category_map(bp_category: &str) -> &str {
    match bp_category {
        "Alimentation" => "Groceries",
        "Shopping et services" => "Shopping",
        "Logement - maison" => "Housing",
        "Loisirs et vacances" => "Life & Entertainment",
        "Banque et assurances" => "Bank Fees",
        "Transports" => "Transport",
        "Sante" => "Medical & Healthcare",
        "Abonnements" => "Software Subscriptions",
        "Salaires et revenus" => "Income",
        _ => {
            tracing::debug!("category {bp_category} is not recognized");
            "Uncategorized"
        }
    }
}

pub fn add_operation(
    conn: &Connection,
    account_id: i64,
    category_id: i64,
    operation: &Bp,
) -> Result<usize> {
    let date_str = operation
        .date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    let rows = conn.execute(
        "INSERT INTO record (createdAt, updatedAt, label, amount, date, \
                accountId, categoryId, tags, isInProgress, isIncome, \
                isTransfer) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, 0, ?, 0)",
        params![
            now,
            now,
            operation.label,
            operation.amount.abs().to_f64(),
            date_str,
            account_id,
            category_id,
            operation.amount.is_sign_positive()
        ],
    )?;
    Ok(rows)
}

pub fn ensure_uncategorized(conn: &Connection) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM category WHERE name = 'Uncategorized' AND deletedAt IS NULL",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    conn.execute(
        "INSERT INTO category (createdAt, updatedAt, name, nature, color) VALUES (?1, ?2, 'Uncategorized', 'WANT', 'grey50')",
        params![now, now],
    )?;
    Ok(conn.last_insert_rowid())
}
