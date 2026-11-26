// src/api/docs.rs: the page that lists every call and lets you try it (Swagger UI)
//
//   /docs           the page
//   /openapi.json   the same list as data, for other tools
//
// The list is built from the notes written above each handler, so it can't drift away from the
// code. These two addresses are the only ones that don't need the key: a browser can't send a
// key to load a page, and they show only the *shape* of the API, never a branch. The calls you
// try from the page still need the key (use the "Authorize" button and paste it once).

use super::branches::{self, BranchChanges, BranchInput, BranchView};
use super::error::ErrorBody;
use super::provinces::{self, ChangeRatesBody, ProvinceResponse, TaxRatesResponse};
use super::request::StatusInput;
use axum::Router;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

/// Adds the "bearer key" lock to the list, so the page knows every call needs it.
struct BearerKey;

impl Modify for BearerKey {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some("The API_KEY from your .env file."))
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Quoto API",
        description = "Branch and province tax-rate operations for the Quoto system."
    ),
    paths(
        branches::list,
        branches::read,
        branches::create,
        branches::replace,
        branches::change,
        branches::change_status,
        provinces::list,
        provinces::read,
        provinces::change_rates
    ),
    components(schemas(
        BranchChanges,
        BranchInput,
        BranchView,
        ProvinceResponse,
        TaxRatesResponse,
        ChangeRatesBody,
        StatusInput,
        ErrorBody
    )),
    modifiers(&BearerKey),
    security(("api_key" = []))
)]
pub(super) struct ApiDoc;

/// The two documentation addresses. Nothing here needs the database. (`auth::is_public` is the
/// one place that lets them through without the key; keep the two in step.)
pub(super) fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    SwaggerUi::new("/docs")
        .url("/openapi.json", ApiDoc::openapi())
        .into()
}
