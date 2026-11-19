use std::collections::{BTreeMap, HashSet};

use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{Response, StatusCode, header},
};
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppError, state::AppState};

#[derive(Deserialize)]
pub struct ReportQuery {
    from: DateTime<FixedOffset>,
    to: DateTime<FixedOffset>,
    user_id: Option<Uuid>,
    project_id: Option<Uuid>,
}

#[derive(FromRow)]
struct ReportRow {
    user_id: Uuid,
    user_name: String,
    project_name: String,
    title: String,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ReportUser {
    id: Uuid,
    name: String,
}

#[derive(Serialize, FromRow)]
pub struct SummaryRow {
    user_id: Uuid,
    project_id: Uuid,
    hours: f64,
}

fn validate_range(query: &ReportQuery) -> Result<(DateTime<Utc>, DateTime<Utc>), AppError> {
    let from = query.from.with_timezone(&Utc);
    let to = query.to.with_timezone(&Utc);
    if to <= from || to - from > chrono::Duration::days(366) {
        return Err(AppError::Validation(
            "o período deve ter no máximo 366 dias".into(),
        ));
    }
    Ok((from, to))
}

pub async fn summary(
    State(state): State<AppState>,
    admin: CurrentUser,
    Query(query): Query<ReportQuery>,
) -> Result<Json<Vec<SummaryRow>>, AppError> {
    admin.require_admin()?;
    let (from, to) = validate_range(&query)?;
    let rows = sqlx::query_as::<_, SummaryRow>(
        "SELECT user_id, project_id,
                sum(extract(epoch from (least(ends_at,$2) - greatest(starts_at,$1))) / 3600.0)::float8 AS hours
         FROM time_entries
         WHERE starts_at < $2 AND ends_at > $1
           AND ($3::uuid IS NULL OR user_id=$3)
           AND ($4::uuid IS NULL OR project_id=$4)
         GROUP BY user_id, project_id ORDER BY user_id, project_id LIMIT 10001",
    )
    .bind(from).bind(to).bind(query.user_id).bind(query.project_id)
    .fetch_all(&state.db).await?;
    if rows.len() > 10_000 {
        return Err(AppError::Validation(
            "relatório grande demais; reduza o período".into(),
        ));
    }
    tracing::info!(target: "audit", actor_id=%admin.id, action="report_summary", result="success", rows=rows.len(), "security event");
    Ok(Json(rows))
}

pub async fn xlsx(
    State(state): State<AppState>,
    admin: CurrentUser,
    Query(query): Query<ReportQuery>,
) -> Result<Response<Body>, AppError> {
    admin.require_admin()?;
    let _permit = state
        .exports
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::RateLimited)?;
    let (from, to) = validate_range(&query)?;
    let report_users = sqlx::query_as::<_, ReportUser>(
        "SELECT id,name FROM users WHERE active=true AND ($1::uuid IS NULL OR id=$1) ORDER BY name",
    )
    .bind(query.user_id)
    .fetch_all(&state.db)
    .await?;
    if report_users.len() > 1000 {
        return Err(AppError::Validation(
            "há usuários demais para uma planilha".into(),
        ));
    }
    let rows = sqlx::query_as::<_, ReportRow>(
        "SELECT e.user_id, u.name AS user_name, p.name AS project_name, e.title, e.starts_at, e.ends_at
         FROM time_entries e
         JOIN users u ON u.id=e.user_id
         JOIN projects p ON p.id=e.project_id
         WHERE e.starts_at < $2 AND e.ends_at > $1
           AND ($3::uuid IS NULL OR e.user_id=$3)
           AND ($4::uuid IS NULL OR e.project_id=$4)
         ORDER BY u.name, e.starts_at LIMIT 10001",
    )
    .bind(from).bind(to).bind(query.user_id).bind(query.project_id)
    .fetch_all(&state.db).await?;
    if rows.len() > 10_000 {
        return Err(AppError::Validation(
            "planilha grande demais; reduza o período".into(),
        ));
    }
    let rows_count = rows.len();

    let mut grouped: BTreeMap<Uuid, (String, Vec<ReportRow>)> = report_users
        .into_iter()
        .map(|user| (user.id, (user.name, Vec::new())))
        .collect();
    for row in rows {
        grouped
            .entry(row.user_id)
            .or_insert_with(|| (row.user_name.clone(), Vec::new()))
            .1
            .push(row);
    }
    if grouped.is_empty() {
        grouped.insert(Uuid::nil(), ("Sem registros".into(), Vec::new()));
    }

    let bytes = workbook(grouped, *query.from.offset());
    if bytes.len() > 25 * 1024 * 1024 {
        return Err(AppError::Validation(
            "planilha excede 25 MB; reduza o período".into(),
        ));
    }
    tracing::info!(target: "audit", actor_id=%admin.id, action="export_xlsx", result="success", rows=rows_count, "security event");
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=horasculo.xlsx",
        )
        .body(Body::from(bytes))
        .map_err(|_| AppError::Internal)
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sheet_name(value: &str, used: &mut HashSet<String>) -> String {
    let clean: String = value
        .chars()
        .map(|c| if "\\/?*[]:".contains(c) { ' ' } else { c })
        .collect();
    let base: String = clean.trim().chars().take(31).collect();
    let base = if base.is_empty() {
        "Usuário".to_owned()
    } else {
        base
    };
    let mut candidate = base.clone();
    let mut number = 2;
    while used.contains(&candidate) {
        let suffix = format!(" ({number})");
        number += 1;
        candidate = format!(
            "{}{}",
            base.chars()
                .take(31usize.saturating_sub(suffix.len()))
                .collect::<String>(),
            suffix
        );
    }
    used.insert(candidate.clone());
    candidate
}

fn cell(value: &str) -> String {
    format!("<c t=\"inlineStr\"><is><t>{}</t></is></c>", xml(value))
}

fn workbook(grouped: BTreeMap<Uuid, (String, Vec<ReportRow>)>, offset: FixedOffset) -> Vec<u8> {
    let mut zip = Zip::default();
    let mut used = HashSet::new();
    let mut sheets = Vec::new();
    for (index, (_, (name, rows))) in grouped.into_iter().enumerate() {
        let name = sheet_name(&name, &mut used);
        let mut body = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
        );
        body.push_str("<row>");
        for heading in [
            "Descrição",
            "Projeto",
            "Data",
            "Início",
            "Fim",
            "Duração (h)",
        ] {
            body.push_str(&cell(heading));
        }
        body.push_str("</row>");
        for row in rows {
            let start = row.starts_at.with_timezone(&offset);
            let end = row.ends_at.with_timezone(&offset);
            let hours = (row.ends_at - row.starts_at).num_minutes() as f64 / 60.0;
            body.push_str("<row>");
            let values = vec![
                row.title,
                row.project_name,
                start.format("%d/%m/%Y").to_string(),
                start.format("%H:%M").to_string(),
                end.format("%H:%M").to_string(),
                format!("{hours:.2}"),
            ];
            for value in values {
                body.push_str(&cell(&value));
            }
            body.push_str("</row>");
        }
        body.push_str("</sheetData></worksheet>");
        zip.add(
            format!("xl/worksheets/sheet{}.xml", index + 1),
            body.into_bytes(),
        );
        sheets.push(name);
    }
    let workbook_sheets = sheets
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                "<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>",
                xml(n),
                i + 1,
                i + 1
            )
        })
        .collect::<String>();
    zip.add("xl/workbook.xml".into(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>{workbook_sheets}</sheets></workbook>").into_bytes());
    let rels = sheets.iter().enumerate().map(|(i,_)| format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{}.xml\"/>",i+1,i+1)).collect::<String>();
    zip.add("xl/_rels/workbook.xml.rels".into(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{rels}</Relationships>").into_bytes());
    zip.add("_rels/.rels".into(), b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".to_vec());
    let overrides = sheets.iter().enumerate().map(|(i,_)| format!("<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>",i+1)).collect::<String>();
    zip.add("[Content_Types].xml".into(), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>{overrides}</Types>").into_bytes());
    zip.finish()
}

#[derive(Default)]
struct Zip {
    files: Vec<(String, Vec<u8>)>,
}
impl Zip {
    fn add(&mut self, name: String, data: Vec<u8>) {
        self.files.push((name, data));
    }
    fn finish(self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut central = Vec::new();
        for (name, data) in &self.files {
            let offset = output.len() as u32;
            let crc = crc32(data);
            local(&mut output, name, data, crc);
            central_entry(&mut central, name, data, crc, offset);
        }
        let central_offset = output.len() as u32;
        let central_size = central.len() as u32;
        output.extend(central);
        output.extend(0x06054b50u32.to_le_bytes());
        output.extend(0u16.to_le_bytes());
        output.extend(0u16.to_le_bytes());
        output.extend((self.files.len() as u16).to_le_bytes());
        output.extend((self.files.len() as u16).to_le_bytes());
        output.extend(central_size.to_le_bytes());
        output.extend(central_offset.to_le_bytes());
        output.extend(0u16.to_le_bytes());
        output
    }
}
fn local(out: &mut Vec<u8>, name: &str, data: &[u8], crc: u32) {
    out.extend(0x04034b50u32.to_le_bytes());
    out.extend(20u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(crc.to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(name.as_bytes());
    out.extend(data);
}
fn central_entry(out: &mut Vec<u8>, name: &str, data: &[u8], crc: u32, offset: u32) {
    out.extend(0x02014b50u32.to_le_bytes());
    out.extend(20u16.to_le_bytes());
    out.extend(20u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(crc.to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend(offset.to_le_bytes());
    out.extend(name.as_bytes());
}
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_zip_based_xlsx_and_safe_unique_sheet_names() {
        let mut used = HashSet::new();
        assert_eq!(sheet_name("Equipe/Produto", &mut used), "Equipe Produto");
        assert_eq!(
            sheet_name("Equipe/Produto", &mut used),
            "Equipe Produto (2)"
        );

        let mut grouped = BTreeMap::new();
        grouped.insert(Uuid::nil(), ("Pessoa".into(), Vec::new()));
        let bytes = workbook(grouped, FixedOffset::east_opt(0).unwrap());
        assert_eq!(&bytes[0..4], b"PK\x03\x04");
        assert!(
            bytes
                .windows("xl/workbook.xml".len())
                .any(|part| part == b"xl/workbook.xml")
        );
    }
}
