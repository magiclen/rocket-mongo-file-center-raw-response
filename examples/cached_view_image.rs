#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

extern crate rocket_mongo_file_center_raw_response;

extern crate validators;

use std::path::Path;

use rocket_mongo_file_center_raw_response::{FileCenterRawResponse, EtagIfNoneMatch};
use rocket_mongo_file_center_raw_response::mongo_file_center::{FileCenter, FileCenterError, mime};

use rocket::request::State;

use validators::short_crypt_url_component::ShortCryptUrlComponent;

const HOST: &str = "localhost";
const PORT: u16 = 27017;

#[get("/<id_token>")]
fn view(etag_if_none_match: EtagIfNoneMatch, file_center: State<FileCenter>, id_token: ShortCryptUrlComponent) -> Result<FileCenterRawResponse, FileCenterError> {
    let id = file_center.decrypt_id_token(id_token.get_short_crypt_url_component())?;

    let id_token = id_token.into_string();

    let etag = FileCenterRawResponse::create_etag_by_id_token(id_token);

    FileCenterRawResponse::from_object_id(file_center.inner(), etag_if_none_match, etag, &id, None::<String>)
}

fn main() {
    let database = "test_rocket_mongo_file_center_raw_response";

    let file_center = FileCenter::new(HOST, PORT, database).unwrap();

    let path = Path::join(Path::new("examples"), Path::join(Path::new("images"), "image(貓).jpg"));

    let file = file_center.put_file_by_path(path, None::<String>, Some(mime::IMAGE_JPEG)).unwrap();

    let id_token = file_center.encrypt_id(file.get_object_id());

    println!("The ID token is: {}", id_token);

    println!();

    rocket::ignite().manage(file_center).mount("/", routes![view]).launch();
}