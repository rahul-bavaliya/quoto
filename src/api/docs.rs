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
use super::customers::{self, CustomerChanges, CustomerInput, CustomerView};
use super::equipment::{
    self, EquipmentChanges, EquipmentInput, EquipmentView, NewEquipment, OwnerInput,
};
use super::error::ErrorBody;
use super::quotes::{
    self, LineInput, LineView, NewQuoteInput, PdfFile, QuoteChanges, QuoteCustomerInput,
    QuoteEquipmentInput, QuoteView, RejectInput, TotalsView,
};
use super::repair_orders::{
    self, CancelRepairOrderInput, OpenRepairOrderInput, RepairOrderChanges, RepairOrderMeterView,
    RepairOrderView,
};
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
        description = "Branches, customers, equipment, quotes and repair orders for the Quoto quoting and repair-order system. \
            Nothing is ever erased: a branch or a customer is made inactive, and DELETE on equipment \
            archives the machine, and a quote or a repair order is never erased. Customers and machines identify real people and vehicles: \
            use made-up ones only until real login exists."
    ),
    paths(
        branches::list,
        branches::read,
        branches::create,
        branches::replace,
        branches::change,
        branches::change_status,
        customers::list,
        customers::read,
        customers::create,
        customers::replace,
        customers::change,
        customers::change_status,
        equipment::list,
        equipment::read,
        equipment::create,
        equipment::replace,
        equipment::change,
        equipment::archive,
        equipment::change_status,
        equipment::change_owner,
        quotes::list,
        quotes::read,
        quotes::print,
        quotes::create,
        quotes::change,
        quotes::add_line,
        quotes::add_customer,
        quotes::add_equipment,
        quotes::accept,
        quotes::reject,
        quotes::reopen,
        repair_orders::open,
        repair_orders::list,
        repair_orders::read,
        repair_orders::change,
        repair_orders::start,
        repair_orders::complete,
        repair_orders::cancel
    ),
    components(schemas(
        BranchChanges,
        BranchInput,
        BranchView,
        CustomerChanges,
        CustomerInput,
        CustomerView,
        EquipmentChanges,
        EquipmentInput,
        EquipmentView,
        NewEquipment,
        OwnerInput,
        StatusInput,
        LineInput,
        LineView,
        NewQuoteInput,
        PdfFile,
        QuoteChanges,
        QuoteCustomerInput,
        QuoteEquipmentInput,
        QuoteView,
        RejectInput,
        TotalsView,
        CancelRepairOrderInput,
        OpenRepairOrderInput,
        RepairOrderChanges,
        RepairOrderMeterView,
        RepairOrderView,
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
