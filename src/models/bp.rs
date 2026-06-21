use anyhow::{Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// One row of a Banque Populaire CSV export (`;`-separated, no header).
#[derive(Debug, Clone, PartialEq)]
pub struct Bp {
    pub date: NaiveDate,
    pub label: String,
    pub full_label: String,
    pub reference: String,
    pub detail: String,
    pub op_type: String,
    pub category: String,
    pub subcategory: String,
    /// Debit and credit columns merged; sign carried from the source (- debit, + credit).
    pub amount: Decimal,
    pub date2: NaiveDate,
    pub value_date: NaiveDate,
    pub flag: String,
}

/// Raw positional row as it appears in the CSV; deserialized by the `csv` crate, then
/// converted into the typed [`Bp`] via [`TryFrom`].
#[derive(Debug, serde::Deserialize, Clone)]
pub struct BpRaw {
    date: String,
    label: String,
    full_label: String,
    reference: String,
    detail: String,
    op_type: String,
    category: String,
    subcategory: String,
    debit: String,
    credit: String,
    date2: String,
    value_date: String,
    flag: String,
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%d/%m/%Y").with_context(|| format!("invalid date: {s:?}"))
}

fn parse_amount(s: &str) -> Result<Decimal> {
    let cleaned = s.trim().trim_start_matches('+').replace(',', ".");
    cleaned
        .parse::<Decimal>()
        .with_context(|| format!("invalid amount: {s:?}"))
}

impl TryFrom<BpRaw> for Bp {
    type Error = anyhow::Error;

    fn try_from(r: BpRaw) -> Result<Self> {
        let amount = match (r.debit.trim().is_empty(), r.credit.trim().is_empty()) {
            (false, true) => parse_amount(&r.debit)?,
            (true, false) => parse_amount(&r.credit)?,
            _ => anyhow::bail!(
                "expected exactly one of debit/credit, got {:?}/{:?} for =>\n{:#?}",
                r.debit,
                r.credit,
                r
            ),
        };
        Ok(Bp {
            date: parse_date(&r.date)?,
            label: r.label,
            full_label: r.full_label,
            reference: r.reference,
            detail: r.detail,
            op_type: r.op_type,
            category: r.category,
            subcategory: r.subcategory,
            amount,
            date2: parse_date(&r.date2)?,
            value_date: parse_date(&r.value_date)?,
            flag: r.flag,
        })
    }
}
