use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
};

use bitcoin::Txid;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    Database, Decode, Encode, Postgres, Type,
    encode::IsNull,
    error::BoxDynError,
    postgres::{PgTypeInfo, PgValueRef},
    types::Uuid,
};
pub use url::Url;
use utoipa::{PartialSchema, openapi};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct SocketAddrWrapped(pub SocketAddr);

impl PartialSchema for SocketAddrWrapped {
    fn schema() -> openapi::RefOr<openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::SchemaType::Type(openapi::schema::Type::String))
            .examples(Some(json!(&SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                8080
            )))))
            .into()
    }
}

impl utoipa::ToSchema for SocketAddrWrapped {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("SocketAddr")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxIdWrapped(pub Txid);

impl serde::Serialize for TxIdWrapped {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for TxIdWrapped {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Txid::from_str(&s).map(TxIdWrapped).map_err(serde::de::Error::custom)
    }
}

impl PartialSchema for TxIdWrapped {
    fn schema() -> openapi::RefOr<openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::SchemaType::Type(openapi::schema::Type::String))
            .examples(Some(json!(&TxIdWrapped(
                Txid::from_str("fb0c9ab881331ec7acdd85d79e3197dcaf3f95055af1703aeee87e0d853e81ec",).unwrap()
            ))))
            .into()
    }
}

impl utoipa::ToSchema for TxIdWrapped {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("TransactionId")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct UrlWrapped(pub Url);

impl Type<sqlx::Postgres> for UrlWrapped {
    fn type_info() -> PgTypeInfo {
        <String as Type<sqlx::Postgres>>::type_info()
    }
}

impl Encode<'_, sqlx::Postgres> for UrlWrapped {
    fn encode_by_ref(&self, buf: &mut <Postgres as Database>::ArgumentBuffer<'_>) -> Result<IsNull, BoxDynError> {
        <String as Encode<sqlx::Postgres>>::encode_by_ref(&self.0.to_string(), buf)
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for UrlWrapped {
    fn decode(value: PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(UrlWrapped(Url::from_str(&s)?))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Secp256K1PubkeyWrapped(pub bitcoin::secp256k1::PublicKey);

impl Type<sqlx::Postgres> for Secp256K1PubkeyWrapped {
    fn type_info() -> PgTypeInfo {
        <String as Type<sqlx::Postgres>>::type_info()
    }
}

impl Encode<'_, sqlx::Postgres> for Secp256K1PubkeyWrapped {
    fn encode_by_ref(&self, buf: &mut <Postgres as Database>::ArgumentBuffer<'_>) -> Result<IsNull, BoxDynError> {
        <String as Encode<sqlx::Postgres>>::encode_by_ref(&self.0.to_string(), buf)
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for Secp256K1PubkeyWrapped {
    fn decode(value: PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(Secp256K1PubkeyWrapped(bitcoin::secp256k1::PublicKey::from_str(&s)?))
    }
}

impl PartialSchema for UrlWrapped {
    fn schema() -> openapi::RefOr<openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::SchemaType::Type(openapi::schema::Type::String))
            .examples(Some(json!(&Url::from_str("localhost:8080").unwrap())))
            .into()
    }
}

impl utoipa::ToSchema for UrlWrapped {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Url")
    }
}

pub fn get_uuid() -> Uuid {
    Uuid::new_v4()
}
