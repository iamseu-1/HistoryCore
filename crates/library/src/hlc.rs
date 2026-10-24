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

    /// 벽시계 ms(위 48비트)를 UTC ISO 글로 — `2026-10-24T03:15:00.123Z`(밀리초 세 자리 · `Z`, spec/windows-api.md 4절).
    /// 카운터는 버린다. HistoryTime 엔진 `shelf.rs`의 `iso_of_hlc`와 같은 꼴(원형 그레고리력).
    pub fn utc_iso(self) -> String {
        const DAY_MS: i64 = 86_400_000;
        let ms = self.wall_ms() as i64;
        let (y, mo, d) = civil_from_days(ms.div_euclid(DAY_MS));
        let rest = ms.rem_euclid(DAY_MS);
        let (h, mi, s, milli) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000);
        format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z")
    }
}

/// 1970-01-01부터의 날 수 → (해, 달, 날) — 원형 그레고리력(Howard Hinnant의 `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
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

    /// 받은 판의 시각을 본다 — 그보다 앞서 있지 않으면 시계를 그 뒤로 옮긴다(HLC 받기 규칙).
    pub fn observe(&mut self, seen: Hlc) {
        if seen > self.last {
            self.last = seen;
        }
    }

    pub fn last(&self) -> Hlc {
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::Hlc;

    #[test]
    fn utc_iso_matches_javascript_to_iso_string() {
        // 기대값은 `new Date(ms).toISOString()`(node)으로 셈.
        for (ms, iso) in [
            (0, "1970-01-01T00:00:00.000Z"),
            (1_761_275_700_123, "2025-10-24T03:15:00.123Z"),
            (951_782_400_000, "2000-02-29T00:00:00.000Z"),
            (1_709_210_096_789, "2024-02-29T12:34:56.789Z"),
            (4_102_444_799_999, "2099-12-31T23:59:59.999Z"),
        ] {
            assert_eq!(Hlc::new(ms, 0).utc_iso(), iso);
            assert_eq!(Hlc::new(ms, u16::MAX).utc_iso(), iso, "카운터는 버린다");
        }
    }
}
