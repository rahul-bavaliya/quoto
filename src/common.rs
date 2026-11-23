// Shared domain types that are not owned by one feature.

mod address;
mod location;
mod phone_number;
mod postal_code;

pub use address::Address;
pub use location::Location;
pub use phone_number::PhoneNumber;
pub use postal_code::PostalCode;
