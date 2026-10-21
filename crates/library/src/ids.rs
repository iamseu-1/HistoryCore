use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub Uuid);

        impl $name {
            /// 새 UUIDv7 — 만든 시각이 앞쪽에 들어 있다.
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn parse(text: &str) -> Option<Self> {
                Uuid::parse_str(text).ok().map(Self)
            }
            pub fn from_slice(bytes: &[u8]) -> Option<Self> {
                Uuid::from_slice(bytes).ok().map(Self)
            }
            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(&self.0.as_bytes()[..]))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                Uuid::from_slice(value.as_blob()?).map(Self).map_err(|e| FromSqlError::Other(Box::new(e)))
            }
        }
    };
}

id_type!(
    /// 문서 — 바뀌지 않는 정체(설계 5-1). 제목이 바뀌어도 그대로다.
    DocId
);
id_type!(
    /// 판(설계 5-2).
    RevId
);
id_type!(
    /// 이 서재를 쓰는 기기. 서재 파일마다 하나.
    DeviceId
);
id_type!(
    /// 서재 하나. 기기가 달라도 같은 서재면 같다(P4 동기화).
    LibraryId
);
