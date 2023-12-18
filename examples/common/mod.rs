use std::env;

use validators::prelude::*;

const HOST_URI: &str = "mongodb://localhost:27017";

#[derive(Debug, Clone, Validator)]
#[validator(base64_url(padding(Disallow)))]
pub struct ShortCryptUrlComponent(pub(crate) String);

#[inline]
pub fn get_mongodb_uri(database_name: &str) -> String {
    let mut host_uri = env::var("MONGODB_HOST_URI").unwrap_or_else(|_| HOST_URI.to_string());

    slash_formatter::concat_with_slash_in_place(&mut host_uri, database_name);

    host_uri
}
