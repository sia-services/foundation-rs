use std::collections::HashSet;
use std::fs::File;
use std::future::{Future, Ready, ready};
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Poll, Context};

use actix_web::{Error, HttpMessage};
use actix_web::dev::{ServiceRequest, ServiceResponse, Service, Transform};
use serde::{Serialize, Deserialize};

use jsonwebtoken::{Validation, Algorithm};

use crate::security::SecurityContext;

// use maplit::hashset;

struct Inner {
    _source: Box<Vec<u8>>,   // source of jwt public key
    key:    jsonwebtoken::DecodingKey<'static>, // key reference to source
    validation: Validation
}

impl Inner {
    pub fn new(issuer: String, key_file: PathBuf) -> Self {
        let mut file = File::open(key_file).map_err(|err| format!("Can not open config file: {}", err)).unwrap();
        let mut _source = Vec::with_capacity(1024);
        file.read_to_end(&mut _source).unwrap();

        let _source = Box::new(_source);

        let key = unsafe {
            let source: &'static Vec<u8> = std::mem::transmute(&*_source);
            jsonwebtoken::DecodingKey::from_rsa_pem(source).unwrap()
        };

        let mut validation = Validation::new(Algorithm::RS256);
        // let aud = hashset!{ issuer };
        validation.iss = Some(issuer);
        validation.validate_exp = true;

        Self { _source, key, validation}
    }
}

pub struct IdentityMiddleware<S> {
    service: S,
    inner: Arc<Inner>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp:    usize,     // Expiration time
    iat:    usize,     // Issued at
    iss:    String,    // Issuer
    sub:    String,    // Subject (user-id)
    groups: HashSet<String>, // Roles set
}

impl <S,B> IdentityMiddleware<S>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    fn construct_context(&mut self, req: &ServiceRequest) -> Result<(), String> {
        let auth_header = req.headers().get("Authorization");
        match auth_header {
            Some(auth_header) => {
                let _split: Vec<&str> = auth_header.to_str().unwrap().split("Bearer").collect();
                let token = _split[1].trim();

                let decode_result = jsonwebtoken::decode::<Claims>(token, &self.inner.key, &self.inner.validation);
                match decode_result {
                    Ok(result) => {
                        let claims = result.claims;

                        println!("iss: {}, sub: {}", &claims.iss, &claims.sub);
                        
                        let user_id: u32 = claims.sub.parse().unwrap_or(0);
                        req.extensions_mut().insert(SecurityContext::new(user_id, claims.groups));
                        Ok(())
                    },
                    Err(err) => {
                        Err(format!("Can not decode authorization token: {}", err))
                    }
                }
            },
            None => Ok(())
        }
    }

}

impl<S,B> Service for IdentityMiddleware<S>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        match self.construct_context(&req) {
            Ok(_) => {
                let fut = self.service.call(req);

                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res)
                })
            },
            Err(err) => {
                Box::pin(async { Err(actix_web::error::ErrorBadRequest(err))})
            }
        }
    }

}

#[derive(Clone)]
pub struct IdentityService {
    inner: Arc<Inner>,
}

impl IdentityService {
    pub fn new(issuer: String, key_file: PathBuf) -> Self {
        let inner = Arc::new(Inner::new(issuer, key_file));
        Self { inner }
    }
}

impl <S,B> Transform<S> for IdentityService
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = IdentityMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IdentityMiddleware { service, inner: self.inner.clone() }))
    }
}
