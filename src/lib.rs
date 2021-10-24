/*!
# File Center Raw Response on MongoDB for Rocket Framework

This crate provides response struct used for responding raw data from the File Center on MongoDB with **Etag** cache optionally.

See `examples`.
*/

extern crate tokio_util;

pub extern crate mongo_file_center;

extern crate rocket_etag_if_none_match;

extern crate rocket;
extern crate url_escape;

use std::io::Cursor;

use tokio_util::io::StreamReader;

use mongo_file_center::bson::oid::ObjectId;
use mongo_file_center::{FileCenter, FileCenterError, FileData, FileItem};

pub use rocket_etag_if_none_match::entity_tag::EntityTag;
pub use rocket_etag_if_none_match::EtagIfNoneMatch;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

/// The response struct used for responding raw data from the File Center on MongoDB with **Etag** cache.
#[derive(Debug)]
pub struct FileCenterRawResponse {
    etag: Option<EntityTag<'static>>,
    file: Option<(Option<String>, FileItem)>,
}

impl FileCenterRawResponse {
    /// Create a `FileCenterRawResponse` instance from a file item.
    #[inline]
    pub fn from_file_item<S: Into<String>>(
        etag: Option<EntityTag<'static>>,
        file_item: FileItem,
        file_name: Option<S>,
    ) -> FileCenterRawResponse {
        let file_name = file_name.map(|file_name| file_name.into());

        FileCenterRawResponse {
            etag,
            file: Some((file_name, file_item)),
        }
    }

    /// Create a `FileCenterRawResponse` instance from the object ID.
    pub async fn from_object_id<'a, S: Into<String>>(
        file_center: &'a FileCenter,
        client_etag: Option<&EtagIfNoneMatch<'a>>,
        etag: Option<EntityTag<'static>>,
        id: ObjectId,
        file_name: Option<S>,
    ) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let is_etag_match = if let Some(client_etag) = client_etag {
            match etag.as_ref() {
                Some(etag) => client_etag.weak_eq(etag),
                None => false,
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
            let file_item = file_center.get_file_item_by_id(id).await?;

            match file_item {
                Some(file_item) => Ok(Some(Self::from_file_item(etag, file_item, file_name))),
                None => Ok(None),
            }
        }
    }

    /// Create a `FileCenterRawResponse` instance from an ID token. It will force to use the `ETag` cache.
    #[inline]
    pub async fn from_id_token<'a, T: AsRef<str> + Into<String>, S: Into<String>>(
        file_center: &FileCenter,
        client_etag: &EtagIfNoneMatch<'a>,
        id_token: T,
        file_name: Option<S>,
    ) -> Result<Option<FileCenterRawResponse>, FileCenterError> {
        let id = file_center.decrypt_id_token(id_token.as_ref())?;

        let etag = Self::create_etag_by_id_token(id_token);

        Self::from_object_id(file_center, Some(client_etag), Some(etag), id, file_name).await
    }

    /// Given an **id_token**, and turned into an `EntityTag` instance.
    #[inline]
    pub fn create_etag_by_id_token<S: Into<String>>(id_token: S) -> EntityTag<'static> {
        EntityTag::with_string(true, id_token.into()).unwrap()
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

impl<'r, 'o: 'r> Responder<'r, 'o> for FileCenterRawResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let mut response = Response::build();

        match self.file {
            Some((file_name, file_item)) => {
                if let Some(etag) = self.etag {
                    response.raw_header("Etag", etag.to_string());
                }

                let file_name = file_name.as_deref().unwrap_or_else(|| file_item.get_file_name());

                if !file_name.is_empty() {
                    let mut v = String::from("inline; filename*=UTF-8''");

                    url_escape::encode_component_to_string(file_name, &mut v);

                    response.raw_header("Content-Disposition", v);
                }

                response.raw_header("Content-Type", file_item.get_mime_type().to_string());

                let file_size = file_item.get_file_size();

                match file_item.into_file_data() {
                    FileData::Buffer(v) => {
                        response.sized_body(v.len(), Cursor::new(v));
                    }
                    FileData::Stream(g) => {
                        response.raw_header("Content-Length", file_size.to_string());

                        response.streamed_body(StreamReader::new(g));
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
