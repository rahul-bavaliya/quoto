// build.rs: rebuild when a migration file is added or changed.
// Without this, `sqlx::migrate!` keeps the list of files from the last build
// and a brand-new migration would be silently skipped.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
