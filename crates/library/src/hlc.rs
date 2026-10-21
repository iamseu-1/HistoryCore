use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use std::time::{SystemTime, UNIX_EPOCH};

/// 혼합 논리 시각(HLC) — `(벽시계 ms << 16) | 카운터`. 판의 차례를 정한다. 기기 안에서 늘 커진다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hlc(pub u64);

impl Hlc {
    pub fn new(wall_ms: u64, counter: u16) -> Hlc {
        Hlc((wall_ms << 16) | counter as u64)
    }
    pub fn wall_ms(self) -> u64 {
        self.0 >> 16
    }
    pub fn counter(self) -> u16 {
        self.0 as u16
    }
}

impl ToSql for Hlc {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for Hlc {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Hlc(value.as_i64()? as u64))
    }
}

/// 벽시계(1970년부터 ms). 시험은 바꿔 끼운다.
pub type WallClock = Box<dyn FnMut() -> u64 + Send>;

/// 이 기기의 HLC 시계. 벽시계가 뒤로 가면 마지막 시각의 카운터를 올린다.
pub struct Clock {
    last: Hlc,
    wall: WallClock,
}

impl Clock {
    pub fn system() -> WallClock {
        Box::new(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0))
    }

    /// `last`는 저장소에 남은 가장 늦은 판의 시각 — 앱을 다시 켜도 거꾸로 가지 않게.
    pub fn new(last: Hlc, wall: WallClock) -> Clock {
        Clock { last, wall }
    }

    pub fn now(&mut self) -> Hlc {
        let wall = (self.wall)();
        let next = if wall > self.last.wall_ms() {
            Hlc::new(wall, 0)
        } else if self.last.counter() < u16::MAX {
            Hlc::new(self.last.wall_ms(), self.last.counter() + 1)
        } else {
            Hlc::new(self.last.wall_ms() + 1, 0)
        };
        self.last = next;
        next
    }

    pub fn last(&self) -> Hlc {
        self.last
    }
}
