use axum::http::HeaderValue;
use axum::{
    async_trait,
    extract::connect_info::{ConnectInfo, Connected},
    extract::FromRequestParts,
    http::{
        header::HeaderName, header::AUTHORIZATION as AUTHORISATION, request::Parts, StatusCode,
    },
    RequestPartsExt,
};
use axum_forwarded_header::ForwardedHeader;
use kanidm_proto::constants::X_FORWARDED_FOR;
use kanidmd_lib::prelude::{ClientAuthInfo, ClientCertInfo, Source};

use std::net::{IpAddr, SocketAddr};

use crate::https::ServerState;

#[allow(clippy::declare_interior_mutable_const)]
const X_FORWARDED_FOR_HEADER: HeaderName = HeaderName::from_static(X_FORWARDED_FOR);

pub struct TrustedClientIp(pub IpAddr);

#[async_trait]
impl FromRequestParts<ServerState> for TrustedClientIp {
    type Rejection = (StatusCode, &'static str);

    #[instrument(level = "debug", skip(state))]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let ConnectInfo(ClientConnInfo {
            addr,
            client_cert: _,
        }) = parts
            .extract::<ConnectInfo<ClientConnInfo>>()
            .await
            .map_err(|_| {
                error!("Connect info contains invalid data");
                (
                    StatusCode::BAD_REQUEST,
                    "connect info contains invalid data",
                )
            })?;

        let ip_addr = if state.trust_x_forward_for {
            // at this point we have to check "X-Forwarded-For" or "Forwarded"

            let mut forwarded_header: Option<HeaderValue> = None;
            let mut header_name = "";

            if let Some(forwarded) = parts.headers.get(axum::http::header::FORWARDED) {
                let forwarded = match ForwardedHeader::try_from(forwarded.clone()) {
                    Ok(val) => Some(val),
                    Err(err) => {
                        error!("Failed to parse 'Forwarded header': {}", err);
                        None
                    }
                };

                header_name = axum::http::header::FORWARDED.as_str();
            }

            if let Some(xff) = parts.headers.get(X_FORWARDED_FOR_HEADER) {
                forwarded_header = Some(xff.clone());
                header_name = X_FORWARDED_FOR;
            }

            if let Some(forwarded_value) = forwarded_header {
                // header may be comma separated.
                let first = forwarded_value
                    .to_str()
                    .map(|s|
                        // Split on an optional comma, return the first result.
                        s.split(',').next().unwrap_or(s))
                    .map_err(|_| {
                        error!(
                            "Failed to parse {} header, contains invalid data: {}",
                            header_name,
                            forwarded_value.to_str().unwrap_or("")
                        );
                        (StatusCode::BAD_REQUEST, "Forwarded contains invalid data")
                    })?;

                first.parse::<IpAddr>().map_err(|_| {
                    error!(
                        "Failed to parse {} header, contains invalid IP address: {}",
                        header_name,
                        forwarded_value.to_str().unwrap_or("")
                    );
                    (
                        StatusCode::BAD_REQUEST,
                        "Forwarded header contains invalid IP address",
                    )
                })?
            } else {
                addr.ip()
            }
        } else {
            addr.ip()
        };

        Ok(TrustedClientIp(ip_addr))
    }
}

pub struct VerifiedClientInformation(pub ClientAuthInfo);

#[async_trait]
impl FromRequestParts<ServerState> for VerifiedClientInformation {
    type Rejection = (StatusCode, &'static str);

    #[instrument(level = "debug", skip(state))]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let ConnectInfo(ClientConnInfo { addr, client_cert }) = parts
            .extract::<ConnectInfo<ClientConnInfo>>()
            .await
            .map_err(|_| {
                error!("Connect info contains invalid data");
                (
                    StatusCode::BAD_REQUEST,
                    "connect info contains invalid data",
                )
            })?;

        let ip_addr = if state.trust_x_forward_for {
            if let Some(x_forward_for) = parts.headers.get(X_FORWARDED_FOR_HEADER) {
                // X forward for may be comma separated.
                let first = x_forward_for
                    .to_str()
                    .map(|s|
                        // Split on an optional comma, return the first result.
                        s.split(',').next().unwrap_or(s))
                    .map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "X-Forwarded-For contains invalid data",
                        )
                    })?;

                first.parse::<IpAddr>().map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        "X-Forwarded-For contains invalid ip addr",
                    )
                })?
            } else {
                addr.ip()
            }
        } else {
            addr.ip()
        };

        let bearer_token = if let Some(header) = parts.headers.get(AUTHORISATION) {
            header
                .to_str()
                .map_err(|err| {
                    warn!(?err, "Invalid bearer token, ignoring");
                })
                .ok()
                .and_then(|s| s.split_once(' '))
                .map(|(_, s)| s.to_string())
                .or_else(|| {
                    warn!("bearer token format invalid, ignoring");
                    None
                })
        } else {
            None
        };

        Ok(VerifiedClientInformation(ClientAuthInfo {
            source: Source::Https(ip_addr),
            bearer_token,
            client_cert,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct ClientConnInfo {
    pub addr: SocketAddr,
    // Only set if the certificate is VALID
    pub client_cert: Option<ClientCertInfo>,
}

impl Connected<ClientConnInfo> for ClientConnInfo {
    fn connect_info(target: ClientConnInfo) -> Self {
        target
    }
}

impl axum::extract::connect_info::Connected<std::net::SocketAddr> for ClientConnInfo {
    fn connect_info(target: std::net::SocketAddr) -> Self {
        ClientConnInfo {
            addr: target,
            client_cert: None,
        }
    }
}

