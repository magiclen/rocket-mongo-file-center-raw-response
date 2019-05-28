/*!
# File Center Raw Response on MongoDB for Rocket Framework

This crate provides response struct used for responding raw data from the File Center on MongoDB with **Etag** cache optionally.

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
    etag: Option<EntityTag>,
    file: Option<(Option<String>, FileItem)>,
}

impl FileCenterRawResponse {
    /// Create a `FileCenterRawResponse` instance from a file item.
    #[inline]
    pub fn from_file_item<S: Into<String>>(etag: Option<EntityTag>, file_item: FileItem, file_name: Option<S>) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let file_name = file_name.map(|file_name| file_name.into());

        Ok(Some(FileCenterRawResponse {
            etag,
            file: Some((file_name, file_item)),
        }))
    }

    /// Create a `FileCenterRawResponse` instance from the object ID.
    pub fn from_object_id<S: Into<String>>(file_center: &FileCenter, client_etag: Option<EtagIfNoneMatch>, etag: Option<EntityTag>, id: &ObjectId, file_name: Option<S>) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let is_etag_match = if let Some(client_etag) = client_etag {
            match etag.as_ref() {
                Some(etag) => {
                    client_etag.weak_eq(etag)
                }
                None => false
            }
        } else {
            false
        };

        if is_etag_match {
            Ok(Some(FileCenterRawResponse {
                etag: None,
                file: None,
            }))
        } else {
            let file_item = file_center.get_file_item_by_id(id)?;

            match file_item {
                Some(file_item) => {
                    Self::from_file_item(etag, file_item, file_name)
                }
                None => Ok(None)
            }
        }
    }

    /// Create a `FileCenterRawResponse` instance from an ID token.
    #[inline]
    pub fn from_id_token<T: AsRef<str> + Into<String>, S: Into<String>>(file_center: &FileCenter, client_etag: Option<EtagIfNoneMatch>, id_token: T, file_name: Option<S>) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let id = file_center.decrypt_id_token(id_token.as_ref())?;

        let etag = Self::create_etag_by_id_token(id_token);

        Self::from_object_id(file_center, client_etag, Some(etag), &id, file_name)
    }

    /// Given an **id_token**, and turned into an `EntityTag` instance.
    #[inline]
    pub fn create_etag_by_id_token<S: Into<String>>(id_token: S) -> EntityTag {
        EntityTag::new(true, id_token.into())
    }

    #[inline]
    /// Check if the file item is temporary.
    pub fn is_temporary(&self) -> Option<bool> {
        if let Some((_, file_item)) = self.file.as_ref() {
            Some(file_item.get_expiration_time().is_some())
        } else {
            None
        }
    }
}

impl Responder<'static> for FileCenterRawResponse {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        let mut response = Response::build();

        match self.file {
            Some((file_name, file_item)) => {
                if let Some(etag) = self.etag {
                    response.header(ETag(etag));
                }

                let file_name = file_name.as_ref().map(|file_name| file_name.as_str()).unwrap_or(file_item.get_file_name());

                if !file_name.is_empty() {
                    response.raw_header("Content-Disposition", format!("inline; filename*=UTF-8''{}", percent_encoding::percent_encode(file_name.as_bytes(), percent_encoding::QUERY_ENCODE_SET)));
                }

                response.raw_header("Content-Type", file_item.get_mime_type().to_string());

                let file_size = file_item.get_file_size();

                match file_item.into_file_data() {
                    FileData::Collection(v) => {
                        response.sized_body(Cursor::new(v));
                    }
                    FileData::GridFS(g) => {
                        response.raw_header("Content-Length", file_size.to_string());

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