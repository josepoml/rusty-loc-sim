use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::ClientConfig;
use rustls::DigitallySignedStruct;
use rustls_pki_types::pem::PemObject;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};

use super::errors::SslError;

pub async fn ssl_wrap_socket(
    socket: TcpStream,
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<TlsStream<TcpStream>, SslError> {
    let mut cert = Vec::new();
    cert.push(CertificateDer::from_pem_slice(cert_pem)?);
    let key = PrivateKeyDer::from_pem_slice(key_pem)?;

    let mut config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DangerousVerifier))
        .with_client_auth_cert(cert, key)?;

    let domain = ServerName::try_from("localhost")?;
    let connector = TlsConnector::from(Arc::new(config));
    let tls_stream = connector.connect(domain, socket).await?;

    Ok(tls_stream)
}

#[derive(Debug)]
struct DangerousVerifier;

impl ServerCertVerifier for DangerousVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion()) // Bypass verification
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![rustls::SignatureScheme::RSA_PKCS1_SHA256] // Dummy value
    }
}
