// Shared domain types that are not owned by one feature.

mod address;
mod contact;
mod email;
mod language;
mod location;
mod phone_number;
mod postal_code;

pub use address::Address;
pub use contact::Contact;
pub use email::Email;
pub use language::Language;
pub use location::Location;
pub use phone_number::PhoneNumber;
pub use postal_code::PostalCode;
