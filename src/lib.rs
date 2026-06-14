use std::convert::Infallible;
use std::env;
use std::ffi::OsString;

use axum::Router;
use axum::body::Body;
use axum::http::request::Builder;
use axum::http::{self, Request, Response};
use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, stdin, stdout};
use tokio_util::io::ReaderStream;
use tower::ServiceExt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("InvalidServerProtocol")]
    InvalidServerProtocol,
    #[error("InvalidContentLength")]
    InvalidContentLength,
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Axum(#[from] axum::Error),
    #[error(transparent)]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error(transparent)]
    Infallible(#[from] Infallible),
}

pub trait RouterCgi {
    fn cgi(self) -> impl Future<Output = Result<(), Error>>;
}

impl RouterCgi for Router {
    async fn cgi(self) -> Result<(), Error> {
        exec_cgi(self).await
    }
}

pub async fn exec_cgi(router: Router) -> Result<(), Error> {
    let request = read_axum_request(env::vars_os(), stdin())?;
    let response = router.oneshot(request).await?;
    write_axum_response(response, BufWriter::new(stdout())).await
}

fn read_axum_request(
    env: impl Iterator<Item = (OsString, OsString)>,
    reader: impl AsyncRead + Send + 'static,
) -> Result<Request<Body>, Error> {
    let request = request_from_map(env)?;
    let axum_request = request.map(|info| {
        info.map_or_else(Body::empty, move |content_length| {
            let stdin = reader.take(content_length);
            let stream = ReaderStream::new(stdin);
            Body::from_stream(stream)
        })
    });
    Ok(axum_request)
}

async fn write_axum_response(
    response: Response<Body>,
    mut writer: impl AsyncWrite + Unpin,
) -> Result<(), Error> {
    // Write the response into stdout
    let (parts, body) = response.into_parts();

    // Status
    writer.write_all(b"status: ").await?;
    writer.write_all(parts.status.as_str().as_bytes()).await?;
    writer.write_all(b" ").await?;
    writer
        .write_all(
            parts
                .status
                .canonical_reason()
                .unwrap_or_default()
                .as_bytes(),
        )
        .await?;
    writer.write_all(b"\n").await?;

    // Headers
    for (name, value) in parts.headers {
        if let Some(name) = name {
            writer.write_all(name.as_ref()).await?;
            writer.write_all(b": ").await?;
            writer.write_all(value.as_ref()).await?;
            writer.write_all(b"\n").await?;
        }
    }

    // Blank line
    writer.write_all(b"\n").await?;

    // Body
    let mut body = body.into_data_stream();
    while let Some(chunk) = body.next().await.transpose()? {
        writer.write_all(&chunk).await?;
    }

    writer.flush().await?;
    Ok(())
}

fn request_from_map(
    env: impl Iterator<Item = (OsString, OsString)>,
) -> Result<Request<Option<u64>>, Error> {
    let mut builder = Builder::new();
    let mut content_length: Option<u64> = None;
    for (key, value) in env {
        match key.to_string_lossy().as_ref() {
            "REQUEST_METHOD" => {
                builder = builder.method(value.to_string_lossy().into_owned().as_str());
            }
            "REQUEST_URI" => builder = builder.uri(value.to_string_lossy().into_owned()),
            "SERVER_PROTOCOL" => {
                let version = match value.to_string_lossy().as_ref() {
                    "HTTP/0.9" => Ok(http::Version::HTTP_09),
                    "HTTP/1.0" => Ok(http::Version::HTTP_10),
                    "HTTP/1.1" => Ok(http::Version::HTTP_11),
                    "HTTP/2.0" => Ok(http::Version::HTTP_2),
                    "HTTP/3.0" => Ok(http::Version::HTTP_3),
                    _ => Err(Error::InvalidServerProtocol),
                }?;
                builder = builder.version(version);
            }
            "CONTENT_LENGTH" => {
                let value = value
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| Error::InvalidContentLength)?;
                content_length = Some(value);
                builder = builder.header(
                    http::header::CONTENT_LENGTH,
                    http::header::HeaderValue::from(value),
                );
            }
            "CONTENT_TYPE" => {
                builder = builder.header(
                    http::header::CONTENT_TYPE,
                    http::header::HeaderValue::from_bytes(value.as_encoded_bytes())?,
                );
            }
            key_str if key_str.starts_with("HTTP_") => {
                let header_name_str = key_str["HTTP_".len()..].to_lowercase().replace('_', "-");
                builder = builder.header(
                    http::header::HeaderName::from_bytes(header_name_str.as_bytes())?,
                    http::header::HeaderValue::from_bytes(value.as_encoded_bytes())?,
                );
            }
            _ => {}
        }
    }

    Ok(builder.body(content_length)?)
}
