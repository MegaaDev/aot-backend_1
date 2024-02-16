use std::{
    env,
    future::{ready, Ready},
};

use crate::api::{error::AuthError, RedisPool};
use actix_session::SessionExt;
use actix_web::{dev::Payload, web::Data, FromRequest, HttpRequest};
use anyhow::Result;
use base64::decode as decode_config;
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use redis::Commands;

use super::TokenClaims;

pub struct AuthUser(pub i32);

impl FromRequest for AuthUser {
    type Error = AuthError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let session = req.get_session();

        let redis_pool: Data<RedisPool> = req.app_data::<Data<RedisPool>>().unwrap().clone();
        let mut redis_conn = match redis_pool.get() {
            Ok(conn) => conn,
            Err(_) => return ready(Err(Self::Error::Session)),
        };

        let auth_token: String = match session.get::<String>("token") {
            Ok(auth_token) => match auth_token {
                Some(token) => token,
                None => return ready(Err(Self::Error::Session)),
            },
            Err(_) => return ready(Err(Self::Error::Session)),
        };

        if auth_token.is_empty() {
            return ready(Err(Self::Error::Session));
        }
        let splitted_token = auth_token.split('.').collect::<Vec<&str>>();

        if splitted_token.len() != 3 {
            return ready(Err(Self::Error::Session));
        }

        let is_logout = req.path() == "/user/logout";

        let middle_part_of_jwt = splitted_token[1];

        let secret: String = env::var("COOKIE_KEY").unwrap_or("".to_string());

        let mut token_err = false;

        let token = match decode::<TokenClaims>(
            &auth_token,
            &DecodingKey::from_secret(secret.as_str().as_ref()),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(token) => token,
            Err(e) => {
                token_err = true;
                TokenData {
                    claims: TokenClaims {
                        id: -1,
                        device: e.to_string(),
                        iat: 0,
                        exp: 0,
                    },
                    header: Default::default(),
                }
            }
        };

        if token_err && is_logout {
            let decoded = decode_config(middle_part_of_jwt).unwrap_or([0; 0].to_vec());
            // Convert the decoded bytes into a UTF-8 string
            let payload = String::from_utf8(decoded).unwrap_or("".to_string());

            let payload = serde_json::from_str::<TokenClaims>(&payload)
                .map_err(|_| Self::Error::Session)
                .unwrap_or(TokenClaims {
                    id: -1,
                    device: "".to_string(),
                    iat: 0,
                    exp: 0,
                });
            let user_id = payload.id;
            return ready(Ok(AuthUser(user_id)));
        }

        let user_id = token.claims.id;
        let device = token.claims.device;
        let device_from_token: String = match redis_conn.get(user_id) {
            Ok(device_id) => device_id,
            Err(_) => return ready(Err(Self::Error::Session)),
        };
        if device != *device_from_token {
            ready(Err(Self::Error::Session))
        } else {
            ready(Ok(AuthUser(user_id)))
        }
    }
}
