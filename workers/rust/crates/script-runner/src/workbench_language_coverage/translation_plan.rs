use super::Report;
use crate::native_time::utc_iso_timestamp;
use serde::Serialize;

type RunnerResult<T> = Result<T, String>;

const PLAN_SCHEMA: &str = "kyuubiki.language-pack-translation-plan/v1";

#[derive(Serialize)]
pub(super) struct TranslationPlan {
    schema_version: &'static str,
    #[serde(rename = "generatedAt")]
    generated_at: String,
    #[serde(rename = "requiredKeys")]
    required_keys: usize,
    #[serde(rename = "completeLanguages")]
    pub(super) complete_languages: Vec<String>,
    #[serde(rename = "incompleteLanguages")]
    pub(super) incomplete_languages: Vec<IncompleteLanguage>,
    pub(super) queue: Vec<QueueEntry>,
}

#[derive(Serialize)]
pub(super) struct IncompleteLanguage {
    language: String,
    covered: usize,
    required: usize,
    remaining: usize,
}

#[derive(Serialize)]
pub(super) struct QueueEntry {
    pub(super) language: String,
    pub(super) batch: String,
    order: usize,
    pub(super) covered: usize,
    pub(super) required: usize,
    pub(super) remaining: usize,
    pub(super) draft: String,
    template: String,
}

pub(super) fn build_plan(report: &Report, language: Option<&str>) -> RunnerResult<TranslationPlan> {
    if let Some(language) = language
        && !report.rows.iter().any(|row| row.language == language)
    {
        return Err(format!("unknown Workbench language: {language}"));
    }
    let mut queue = report
        .rows
        .iter()
        .filter(|row| language.is_none_or(|language| row.language == language))
        .flat_map(|row| {
            report
                .batches
                .iter()
                .enumerate()
                .filter_map(move |(order, batch)| {
                    let covered = batch
                        .coverage
                        .iter()
                        .find(|entry| entry.language == row.language)
                        .map(|entry| entry.covered)
                        .unwrap_or_default();
                    let remaining = batch.required - covered;
                    (remaining > 0).then(|| QueueEntry {
                        language: row.language.clone(),
                        batch: batch.id.clone(),
                        order,
                        covered,
                        required: batch.required,
                        remaining,
                        draft: format!(
                            "tmp/language-pack-translation-drafts/{}-{}.json",
                            row.language, batch.id
                        ),
                        template: format!(
                            "tmp/language-pack-translation-batches/{}-{}.json",
                            row.language, batch.id
                        ),
                    })
                })
        })
        .collect::<Vec<_>>();
    queue.sort_by(|left, right| {
        right
            .remaining
            .cmp(&left.remaining)
            .then(left.language.cmp(&right.language))
            .then(left.order.cmp(&right.order))
    });
    Ok(TranslationPlan {
        schema_version: PLAN_SCHEMA,
        generated_at: utc_iso_timestamp(),
        required_keys: report.required_keys.len(),
        complete_languages: report
            .rows
            .iter()
            .filter(|row| row.covered == row.required)
            .map(|row| row.language.clone())
            .collect(),
        incomplete_languages: report
            .rows
            .iter()
            .filter(|row| row.covered != row.required)
            .map(|row| IncompleteLanguage {
                language: row.language.clone(),
                covered: row.covered,
                required: row.required,
                remaining: row.required - row.covered,
            })
            .collect(),
        queue,
    })
}
