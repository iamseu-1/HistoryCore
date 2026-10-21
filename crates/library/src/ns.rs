use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

/// 서재의 이름공간(설계 5-1). 숫자는 저장소 값, `code()`는 다리 값이다 — 둘 다 바꾸지 않는다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Namespace {
    Doc = 0,
    Category = 1,
    Template = 2,
    File = 3,
    Front = 4,
}

/// 링크에서 이름공간으로 읽히는 접두어 — 서재 밖 자료(연표 · 字 · 책)도 든다. 제목이 이것과 `:`로 시작하면 안 된다.
pub const RESERVED_PREFIXES: [&str; 8] = ["문서", "분류", "틀", "파일", "대문", "연표", "字", "책"];

impl Namespace {
    pub const ALL: [Namespace; 5] = [Namespace::Doc, Namespace::Category, Namespace::Template, Namespace::File, Namespace::Front];

    pub fn code(self) -> &'static str {
        match self {
            Namespace::Doc => "doc",
            Namespace::Category => "category",
            Namespace::Template => "template",
            Namespace::File => "file",
            Namespace::Front => "front",
        }
    }

    pub fn from_code(code: &str) -> Option<Namespace> {
        Self::ALL.into_iter().find(|ns| ns.code() == code)
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Namespace::Doc => "문서",
            Namespace::Category => "분류",
            Namespace::Template => "틀",
            Namespace::File => "파일",
            Namespace::Front => "대문",
        }
    }

    pub fn from_i64(value: i64) -> Option<Namespace> {
        Self::ALL.into_iter().find(|ns| *ns as i64 == value)
    }

    /// 본문 링크에 적는 꼴 — 일반 문서는 제목만, 나머지는 `분류:제목`.
    pub fn link_title(self, title: &str) -> String {
        if self == Namespace::Doc {
            title.to_string()
        } else {
            format!("{}:{title}", self.prefix())
        }
    }
}

impl ToSql for Namespace {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i64))
    }
}

impl FromSql for Namespace {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let v = value.as_i64()?;
        Namespace::from_i64(v).ok_or(FromSqlError::OutOfRange(v))
    }
}
