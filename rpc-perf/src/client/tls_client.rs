// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::client::*;
use crate::session::{Session, TLSSession};

use mio::Token;
use rustls::ClientConfig;
use slab::Slab;

use std::fs;
use std::io::BufReader;
use std::sync::Arc;

/// A structure which represents a `Client` across TLS protected `Session`s
pub struct TLSClient {
    common: Common,
    sessions: Slab<TLSSession>,
    tls_config: Option<ClientConfig>,
}

impl TLSClient {
    /// Creates a new `TLSClient` which sends requests from the queue and parses responses
    pub fn new(id: usize, codec: Box<dyn Codec>) -> TLSClient {
        Self {
            common: Common::new(id, codec),
            sessions: Slab::new(),
            tls_config: None,
        }
    }

    // load certificate from file
    fn load_certs(&self, filename: &str) -> Vec<rustls::Certificate> {
        let certfile = fs::File::open(filename).expect("cannot open certificate file");
        let mut reader = BufReader::new(certfile);
        rustls::internal::pemfile::certs(&mut reader).unwrap()
    }

    // load key from file
    fn load_private_key(&self, filename: &str) -> rustls::PrivateKey {
        let keyfile = fs::File::open(filename).expect("cannot open private key file");
        let mut reader = BufReader::new(keyfile);
        let keys = rustls::internal::pemfile::pkcs8_private_keys(&mut reader).unwrap();
        assert_eq!(keys.len(), 1);
        keys[0].clone()
    }

    /// load the client key and certificate from files
    pub fn load_key_and_cert(&mut self, keyfile: &str, certsfile: &str) {
        let certs = self.load_certs(certsfile);
        let privkey = self.load_private_key(keyfile);

        let mut tls_config = self
            .tls_config
            .take()
            .unwrap_or_else(rustls::ClientConfig::new);
        tls_config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
        tls_config.set_single_client_cert(certs, privkey);
        self.tls_config = Some(tls_config);
    }

    /// load the certificate authority from file
    pub fn load_ca(&mut self, cafile: &str) {
        let certfile = fs::File::open(&cafile).expect("Cannot open CA file");
        let mut reader = BufReader::new(certfile);

        {
            if self.tls_config.is_none() {
                self.tls_config = Some(rustls::ClientConfig::new());
            }
        }
        if let Some(ref mut tls_config) = self.tls_config {
            tls_config.root_store.add_pem_file(&mut reader).unwrap();
        }
    }
}

impl Client for TLSClient {
    fn add_endpoint(&mut self, endpoint: &SocketAddr) {
        debug!("adding endpoint: {}", endpoint);
        for _ in 0..self.poolsize() {
            let mut session =
                TLSSession::new(endpoint, self.tls_config.clone().expect("no tls config"));
            session.set_nodelay(self.tcp_nodelay());
            let token = self.sessions.insert(session);
            self.connect_enqueue(mio::Token(token));
        }
        self.connect_shuffle();
    }

    fn common(&self) -> &Common {
        &self.common
    }

    fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    fn session(&self, token: Token) -> &dyn Session {
        &self.sessions[token.into()]
    }

    fn session_mut(&mut self, token: Token) -> &mut dyn Session {
        &mut self.sessions[token.into()]
    }

    fn does_negotiate(&self) -> bool {
        true
    }

    fn prepare_request(&mut self, token: Token, rng: &mut ThreadRng) {
        self.common
            .encode(self.sessions[token.into()].write_buf(), rng)
    }
}

// this is necessary to bypass certificate validation
pub struct NoCertificateVerification {}

impl rustls::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _roots: &rustls::RootCertStore,
        _presented_certs: &[rustls::Certificate],
        _dns_name: webpki::DNSNameRef,
        _ocsp: &[u8],
    ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        Ok(rustls::ServerCertVerified::assertion())
    }
}
