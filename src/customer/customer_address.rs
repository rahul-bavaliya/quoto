// src/customer/address.rs (or src/common/address.rs)

use crate::ProvinceCountry;
use crate::common::Address;
use crate::errors::{AppError, AppResult};

/// Mailing and shipping address pairing for a customer.
/// The province on the active address determines applicable taxes[cite: 2, 3].
#[derive(Debug, PartialEq)]
pub struct CustomerAddress {
    mailing_address: Address,
    use_shipping_as_mailing: bool,
    shipping_address: Option<Address>,
}

impl CustomerAddress {
    /// Creates a `CustomerAddress`.
    /// When `use_shipping_as_mailing` is `true`, shipping uses `mailing_address`
    /// and any passed `shipping_address` is not required.
    /// When `use_shipping_as_mailing` is `false`, a `shipping_address` is required.
    pub fn new(
        mailing_address: Address,
        use_shipping_as_mailing: bool,
        shipping_address: Option<Address>,
    ) -> AppResult<Self> {
        let resolved_shipping = if use_shipping_as_mailing {
            None
        } else {
            let addr = shipping_address.ok_or(AppError::MissingField {
                field: "shipping address",
            })?;
            Some(addr)
        };

        Ok(Self {
            mailing_address,
            use_shipping_as_mailing,
            shipping_address: resolved_shipping,
        })
    }

    /// Convenience constructor when mailing and shipping use the same address.
    pub fn same_as_mailing(mailing_address: Address) -> Self {
        Self {
            mailing_address,
            use_shipping_as_mailing: true,
            shipping_address: None,
        }
    }

    pub fn mailing_address(&self) -> &Address {
        &self.mailing_address
    }

    pub fn use_shipping_as_mailing(&self) -> bool {
        self.use_shipping_as_mailing
    }

    /// Returns the explicit shipping address if set, or `None` when mirroring mailing.
    pub fn explicit_shipping_address(&self) -> Option<&Address> {
        self.shipping_address.as_ref()
    }

    /// Effective shipping address: `mailing_address` if `use_shipping_as_mailing` is `true`,
    /// otherwise the explicit `shipping_address`.
    pub fn effective_shipping_address(&self) -> &Address {
        if self.use_shipping_as_mailing {
            &self.mailing_address
        } else {
            self.shipping_address
                .as_ref()
                .unwrap_or(&self.mailing_address)
        }
    }

    /// Province follows from the effective shipping address.
    pub fn province_country(&self) -> &ProvinceCountry {
        self.effective_shipping_address().province_country()
    }
}
