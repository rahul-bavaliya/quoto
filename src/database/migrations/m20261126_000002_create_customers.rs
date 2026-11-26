use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Customers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Customers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(
                        ColumnDef::new(Customers::IsBusiness)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Customers::BusinessName).text())
                    .col(ColumnDef::new(Customers::CustomerName).text().not_null())
                    // Mailing Address
                    .col(
                        ColumnDef::new(Customers::MailAddressLine1)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Customers::MailAddressLine2).text())
                    .col(ColumnDef::new(Customers::MailCity).text().not_null())
                    .col(
                        ColumnDef::new(Customers::MailProvinceCode)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Customers::MailCountryCode).text().not_null())
                    .col(ColumnDef::new(Customers::MailPostalCode).text().not_null())
                    .col(
                        ColumnDef::new(Customers::UseShippingAsMailing)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    // Shipping Address
                    .col(ColumnDef::new(Customers::ShipAddressLine1).text())
                    .col(ColumnDef::new(Customers::ShipAddressLine2).text())
                    .col(ColumnDef::new(Customers::ShipCity).text())
                    .col(ColumnDef::new(Customers::ShipProvinceCode).text())
                    .col(ColumnDef::new(Customers::ShipCountryCode).text())
                    .col(ColumnDef::new(Customers::ShipPostalCode).text())
                    // Contact & Preferences
                    .col(ColumnDef::new(Customers::Phone1).text().not_null())
                    .col(ColumnDef::new(Customers::Phone2).text())
                    .col(ColumnDef::new(Customers::Email).text().not_null())
                    .col(ColumnDef::new(Customers::Language).text().default("EN"))
                    .col(ColumnDef::new(Customers::GstRegistrationNo).text())
                    .col(ColumnDef::new(Customers::InsuranceExpDate).timestamp_with_time_zone())
                    .col(ColumnDef::new(Customers::Status).text().not_null())
                    .col(
                        ColumnDef::new(Customers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Customers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
    IsBusiness,
    BusinessName,
    CustomerName,
    MailAddressLine1,
    MailAddressLine2,
    MailCity,
    MailProvinceCode,
    MailCountryCode,
    MailPostalCode,
    UseShippingAsMailing,
    ShipAddressLine1,
    ShipAddressLine2,
    ShipCity,
    ShipProvinceCode,
    ShipCountryCode,
    ShipPostalCode,
    Phone1,
    Phone2,
    Email,
    Language,
    GstRegistrationNo,
    InsuranceExpDate,
    Status,
    CreatedAt,
    UpdatedAt,
}
