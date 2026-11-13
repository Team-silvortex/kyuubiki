use super::{Result, transport, types::segment};
use crate::store_identity::StoreIdentity;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRequest {
    pub query: String,
    pub cursor: Option<String>,
}
#[derive(Deserialize, Serialize)]
pub struct CatalogItem {
    pub kind: String,
    pub asset_id: String,
    pub title: String,
    pub summary: String,
    pub domain: String,
    pub languages: Vec<String>,
    pub latest_version: String,
}
#[derive(Deserialize)]
struct Page {
    catalog_revision: String,
    items: Vec<CatalogItem>,
    next_cursor: Option<String>,
}
#[derive(Serialize)]
pub struct CatalogPage {
    pub issuer: String,
    pub host_platform: String,
    pub catalog_revision: String,
    pub items: Vec<CatalogItem>,
    pub next_cursor: Option<String>,
}
pub(super) fn host_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
fn bounded(value: &str, max: usize) -> bool {
    value.len() <= max && !value.chars().any(char::is_control)
}
pub fn catalog(identity: &StoreIdentity, request: CatalogRequest) -> Result<CatalogPage> {
    if !bounded(&request.query, 200)
        || request
            .cursor
            .as_ref()
            .is_some_and(|c| c.is_empty() || !bounded(c, 8192))
    {
        return Err("invalid_catalog_query");
    }
    let session = identity.store_lease()?;
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("limit", "20")
        .append_pair("q", &request.query);
    if let Some(cursor) = request.cursor {
        query.append_pair("cursor", &cursor);
    }
    let page: Page = transport::get(
        &session,
        &format!("/store/v1/assets?{}", query.finish()),
        false,
    )?;
    if page.catalog_revision.is_empty()
        || !bounded(&page.catalog_revision, 256)
        || page.items.len() > 20
        || page
            .next_cursor
            .as_ref()
            .is_some_and(|c| c.is_empty() || !bounded(c, 8192))
    {
        return Err("invalid_store_response");
    }
    let mut previous = None;
    for item in &page.items {
        let key = (&item.kind, &item.asset_id);
        if ![
            "operator",
            "workflow_template",
            "frontend_dsl_template",
            "language_pack",
        ]
        .contains(&item.kind.as_str())
            || !segment(&item.asset_id)
            || !segment(&item.latest_version)
            || !item
                .latest_version
                .starts_with(|c: char| c.is_ascii_digit())
            || !bounded(&item.title, 512)
            || !bounded(&item.summary, 4096)
            || !bounded(&item.domain, 100)
            || item.languages.len() > 100
            || item.languages.iter().any(|l| !bounded(l, 100))
            || previous.is_some_and(|p| p >= key)
        {
            return Err("invalid_store_response");
        }
        previous = Some(key);
    }
    Ok(CatalogPage {
        issuer: session.issuer,
        host_platform: host_platform(),
        catalog_revision: page.catalog_revision,
        items: page.items,
        next_cursor: page.next_cursor,
    })
}
