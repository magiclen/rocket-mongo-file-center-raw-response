/*!
# File Center Raw Response on MongoDB for Rocket Framework

This crate provides response struct used for responding raw data from the File Center on MongoDB with **Etag** cache.

See `examples`.
*/

pub extern crate mongo_file_center;
extern crate percent_encoding;
extern crate rocket;
extern crate rocket_etag_if_none_match;

use std::io::Cursor;

use mongo_file_center::{FileCenter, FileItem, FileData, FileCenterError, bson::oid::ObjectId};

pub use rocket_etag_if_none_match::{EntityTag, EtagIfNoneMatch};

use rocket::response::{self, Response, Responder};
use rocket::request::Request;
use rocket::http::{Status, hyper::header::ETag};

/// The response struct used for responding raw data from the File Center on MongoDB with **Etag** cache.
#[derive(Debug)]
pub struct FileCenterRawResponse {
    /// If it is `None`, that means the **Etag** is well-matched.
    pub file_item: Option<FileItem>,
    pub etag: EntityTag,
    pub file_name: Option<String>,
}

impl Responder<'static> for FileCenterRawResponse {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        let mut response = Response::build();

        match self.file_item {
            Some(file_item) => {
                response.header(ETag(self.etag));

                if let Some(file_name) = self.file_name {
                    if !file_name.is_empty() {
                        response.raw_header("Content-Disposition", format!("inline; filename*=UTF-8''{}", percent_encoding::percent_encode(file_name.as_bytes(), percent_encoding::QUERY_ENCODE_SET)));
                    }
                } else {
                    let file_name = file_item.get_file_name();
                    if !file_name.is_empty() {
                        response.raw_header("Content-Disposition", format!("inline; filename*=UTF-8''{}", percent_encoding::percent_encode(file_name.as_bytes(), percent_encoding::QUERY_ENCODE_SET)));
                    }
                }

                response.raw_header("Content-Type", file_item.get_mime_type().to_string())
                    .raw_header("Content-Length", file_item.get_file_size().to_string());

                match file_item.into_file_data() {
                    FileData::Collection(v) => {
                        response.sized_body(Cursor::new(v));
                    }
                    FileData::GridFS(g) => {
                        response.streamed_body(g);
                    }
                }
            }
            None => {
                response.status(Status::NotModified);
            }
        }

        response.ok()
    }
}

impl FileCenterRawResponse {
    /// Create a `FileCenterRawResponse` instance from a file item.
    pub fn from_object_id<S: Into<String>>(file_center: &FileCenter, etag_if_none_match: EtagIfNoneMatch, etag: EntityTag, id: &ObjectId, file_name: Option<S>) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let is_etag_match = etag_if_none_match.weak_eq(&etag);

        if is_etag_match {
            Ok(Some(FileCenterRawResponse {
                file_item: None,
                etag,
                file_name: None,
            }))
        } else {
            let file_item = file_center.get_file_item_by_id(id)?;

            match file_item {
                Some(file_item) => {
                    let file_name = file_name.map(|file_name| file_name.into());
                    Ok(Some(FileCenterRawResponse {
                        file_item: Some(file_item),
                        etag,
                        file_name,
                    }))
                }
                None => Ok(None)
            }
        }
    }

    /// Given an **id_token**, and turned into an `EntityTag` instance.
    pub fn create_etag_by_id_token<S: Into<String>>(id_token: S) -> EntityTag {
        EntityTag::new(true, id_token.into())
    }
}