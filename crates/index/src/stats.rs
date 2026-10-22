//! 대문 모음 · 서재 통계(설계 5-5 「통계 모음」 · 8-2) — 늘 맞게 고쳐 두는 셈과, 뒤에서 조금씩 다시 세는 갈래 사이 링크.
use crate::store::{meta_i64, set_meta};
use crate::tree::{summary_row, Tree};
use history_library::{Counts, DocSummary, Hlc, Namespace, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

pub(crate) const SCHEMA: &str = "
-- 판이 생긴 시(UTC 시각 ÷ 1시간)마다 판 수 — 날 · 달은 부르는 쪽 시간대로 묶는다.
CREATE TABLE stat_edit(hour INTEGER PRIMARY KEY, n INTEGER NOT NULL);
-- 살아 있는 일반 문서가 만들어진 시마다 수.
CREATE TABLE stat_new(hour INTEGER PRIMARY KEY, n INTEGER NOT NULL);
CREATE TABLE source_count(name TEXT PRIMARY KEY, refs INTEGER NOT NULL) WITHOUT ROWID;
CREATE INDEX source_count_refs ON source_count(refs);
CREATE TABLE decade(decade INTEGER PRIMARY KEY, docs INTEGER NOT NULL);
CREATE TABLE group_link(a TEXT NOT NULL, b TEXT NOT NULL, links INTEGER NOT NULL, PRIMARY KEY(a, b)) WITHOUT ROWID;
CREATE TABLE group_link_next(a TEXT NOT NULL, b TEXT NOT NULL, links INTEGER NOT NULL, PRIMARY KEY(a, b)) WITHOUT ROWID;
";

const HOUR: i64 = 3_600_000;
const DAY: i64 = 86_400_000;
/// 다루는 해 하나가 세는 10년 칸의 한계(500년).
const MAX_DECADES: i32 = 50;
/// 갈래 사이 링크를 한 번에 세는 링크 수.
const GROUP_CHUNK: i64 = 2_000;

pub struct HomeOptions {
    pub recent: usize,
    pub wanted: usize,
    /// 쓰는 리듬의 날 수(오늘까지).
    pub days: usize,
    /// 기기 시간대(UTC에서 분). 날 · 달은 이 시간대로 자른다.
    pub tz_minutes: i32,
    /// 「오늘」(시험용). None이면 지금.
    pub today_ms: Option<u64>,
}

impl Default for HomeOptions {
    fn default() -> Self {
        HomeOptions { recent: 10, wanted: 10, days: 84, tz_minutes: 0, today_ms: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayEdits {
    /// `YYYY-MM-DD`
    pub day: String,
    pub edits: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Home {
    pub groups: Vec<Group>,
    pub uncategorized: u64,
    pub recent: Vec<DocSummary>,
    pub wanted: Vec<crate::Wanted>,
    pub rhythm: Vec<DayEdits>,
    pub counts: Counts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthNew {
    /// `YYYY-MM`
    pub month: String,
    pub docs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupLink {
    pub from: String,
    pub to: String,
    pub links: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceCount {
    pub source: String,
    pub refs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Linked {
    pub doc: DocSummary,
    pub refs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecadeCount {
    pub decade: i32,
    pub docs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stats {
    pub monthly_new: Vec<MonthNew>,
    /// 오늘까지 365일.
    pub daily_edits: Vec<DayEdits>,
    pub group_links: Vec<GroupLink>,
    pub top_sources: Vec<SourceCount>,
    pub top_linked: Vec<Linked>,
    pub decades: Vec<DecadeCount>,
    /// 갈래 사이 링크를 마지막으로 다 센 때(벽시계 ms). 0이면 아직.
    pub as_of: u64,
}

pub(crate) fn hour_of(hlc: Hlc) -> i64 {
    hlc.wall_ms() as i64 / HOUR
}

/// 판 시각들을 시마다 센다.
pub(crate) fn count_edits(db: &Connection, hlcs: &[Hlc]) -> Result<()> {
    let mut by: HashMap<i64, i64> = HashMap::new();
    for h in hlcs {
        *by.entry(hour_of(*h)).or_default() += 1;
    }
    for (hour, n) in by {
        db.prepare_cached("INSERT INTO stat_edit(hour, n) VALUES (?1, ?2) ON CONFLICT(hour) DO UPDATE SET n = n + excluded.n")?.execute(params![hour, n])?;
    }
    Ok(())
}

pub(crate) fn bump_new(db: &Connection, created: Hlc, by: i64) -> Result<()> {
    db.prepare_cached("INSERT INTO stat_new(hour, n) VALUES (?1, ?2) ON CONFLICT(hour) DO UPDATE SET n = n + excluded.n")?.execute(params![hour_of(created), by])?;
    db.prepare_cached("DELETE FROM stat_new WHERE hour = ?1 AND n <= 0")?.execute([hour_of(created)])?;
    Ok(())
}

pub(crate) fn bump_source(db: &Connection, name: &str, by: i64) -> Result<()> {
    db.prepare_cached("INSERT INTO source_count(name, refs) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET refs = refs + excluded.refs")?.execute(params![name, by])?;
    db.prepare_cached("DELETE FROM source_count WHERE name = ?1 AND refs <= 0")?.execute([name])?;
    Ok(())
}

/// 다루는 해가 걸친 10년 칸마다(많아야 50칸).
pub(crate) fn bump_years(db: &Connection, y0: i32, y1: i32, by: i64) -> Result<()> {
    let first = y0.div_euclid(10) * 10;
    let last = y1.div_euclid(10) * 10;
    let mut d = first;
    let mut n = 0;
    while d <= last && n < MAX_DECADES {
        db.prepare_cached("INSERT INTO decade(decade, docs) VALUES (?1, ?2) ON CONFLICT(decade) DO UPDATE SET docs = docs + excluded.docs")?.execute(params![d, by])?;
        db.prepare_cached("DELETE FROM decade WHERE decade = ?1 AND docs <= 0")?.execute([d])?;
        d += 10;
        n += 1;
    }
    Ok(())
}

/// 그레고리력 날짜(1970-01-01부터 날 수) — Howard Hinnant의 civil_from_days.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn local_day(ms: i64, tz_minutes: i32) -> i64 {
    (ms + i64::from(tz_minutes) * 60_000).div_euclid(DAY)
}

fn day_name(day: i64) -> String {
    let (y, m, d) = civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_ms(today: Option<u64>) -> i64 {
    today.map(|t| t as i64).unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0))
}

/// 오늘까지 `days`일의 날마다 고친 횟수(빈 날은 0).
fn edits_by_day(db: &Connection, days: usize, tz_minutes: i32, today: Option<u64>) -> Result<Vec<DayEdits>> {
    let days = days.max(1) as i64;
    let today = local_day(now_ms(today), tz_minutes);
    let first = today - days + 1;
    // 시간대가 ±14시간까지 있으니 앞뒤로 하루씩 넉넉히 읽고 날로 묶는다.
    let (lo, hi) = ((first - 1) * 24, (today + 2) * 24);
    let mut by: BTreeMap<i64, u64> = (first..=today).map(|d| (d, 0)).collect();
    let mut st = db.prepare_cached("SELECT hour, n FROM stat_edit WHERE hour >= ?1 AND hour < ?2")?;
    let mut rows = st.query(params![lo, hi])?;
    while let Some(r) = rows.next()? {
        let (hour, n): (i64, i64) = (r.get(0)?, r.get(1)?);
        if let Some(v) = by.get_mut(&local_day(hour * HOUR, tz_minutes)) {
            *v += n.max(0) as u64;
        }
    }
    Ok(by.into_iter().map(|(d, edits)| DayEdits { day: day_name(d), edits }).collect())
}

pub(crate) fn home(db: &Connection, tree: &Tree, counts: Counts, wanted: Vec<crate::Wanted>, o: &HomeOptions) -> Result<Home> {
    let groups = tree.top().into_iter().map(|f| Group { name: f.name, total: f.total }).collect();
    let recent = {
        let mut st = db.prepare_cached("SELECT doc, ns, title, rev, created, updated FROM item WHERE deleted = 0 ORDER BY updated DESC LIMIT ?1")?;
        let rows = st.query_map([o.recent as i64], summary_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let rhythm = edits_by_day(db, o.days, o.tz_minutes, o.today_ms)?;
    Ok(Home { groups, uncategorized: tree.uncategorized(), recent, wanted, rhythm, counts })
}

pub(crate) fn stats(db: &Connection, tz_minutes: i32, today: Option<u64>) -> Result<Stats> {
    let mut months: BTreeMap<(i64, u32), u64> = BTreeMap::new();
    {
        let mut st = db.prepare_cached("SELECT hour, n FROM stat_new")?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let (hour, n): (i64, i64) = (r.get(0)?, r.get(1)?);
            let (y, m, _) = civil(local_day(hour * HOUR, tz_minutes));
            *months.entry((y, m)).or_default() += n.max(0) as u64;
        }
    }
    let monthly_new = months.into_iter().filter(|(_, n)| *n > 0).map(|((y, m), docs)| MonthNew { month: format!("{y:04}-{m:02}"), docs }).collect();
    let daily_edits = edits_by_day(db, 365, tz_minutes, today)?;
    let group_links = {
        let mut st = db.prepare_cached("SELECT a, b, links FROM group_link ORDER BY a, b")?;
        let rows = st.query_map([], |r| Ok(GroupLink { from: r.get(0)?, to: r.get(1)?, links: r.get::<_, i64>(2)? as u64 }))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let top_sources = {
        let mut st = db.prepare_cached("SELECT name, refs FROM source_count ORDER BY refs DESC, name LIMIT 20")?;
        let rows = st.query_map([], |r| Ok(SourceCount { source: r.get(0)?, refs: r.get::<_, i64>(1)? as u64 }))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut top_linked = Vec::new();
    {
        let mut st = db.prepare_cached("SELECT ns, key, refs FROM target WHERE present = 1 ORDER BY refs DESC, ns, key LIMIT 20")?;
        let rows: Vec<(String, String, i64)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
        for (ns, key, refs) in rows {
            let Some(code) = Namespace::ALL.into_iter().find(|n| n.prefix() == ns) else { continue };
            let doc = db
                .prepare_cached("SELECT doc, ns, title, rev, created, updated FROM item WHERE ns = ?1 AND title = ?2 AND deleted = 0 ORDER BY created LIMIT 1")?
                .query_row(params![code, key], summary_row)
                .optional()?;
            if let Some(doc) = doc {
                top_linked.push(Linked { doc, refs: refs as u64 });
            }
        }
    }
    let decades = {
        let mut st = db.prepare_cached("SELECT decade, docs FROM decade WHERE docs > 0 ORDER BY decade")?;
        let rows = st.query_map([], |r| Ok(DecadeCount { decade: r.get(0)?, docs: r.get::<_, i64>(1)? as u64 }))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let as_of = meta_i64(db, "gl_as_of")?.max(0) as u64;
    Ok(Stats { monthly_new, daily_edits, group_links, top_sources, top_linked, decades, as_of })
}

/// 갈래 사이 링크를 다시 세야 하거나 세는 중인가.
pub(crate) fn pending(db: &Connection) -> Result<bool> {
    Ok(meta_i64(db, "stats_dirty")? != 0 || meta_i64(db, "gl_cursor")? > 0)
}

/// 갈래 사이 링크를 `deadline`까지 센다 — 링크 표를 번호 차례로 조금씩 훑어 `group_link_next`에 모으고, 다 세면 바꿔 끼운다.
/// 세는 도중 링크 · 집이 바뀌면 `stats_dirty`가 다시 서서 다음에 처음부터 다시 센다.
pub(crate) fn count_group_links(db: &Connection, tree: &Tree, deadline: Instant) -> Result<()> {
    let mut cursor = meta_i64(db, "gl_cursor")?;
    if cursor <= 0 {
        if meta_i64(db, "stats_dirty")? == 0 {
            return Ok(());
        }
        let tx = db.unchecked_transaction()?;
        tx.execute("DELETE FROM group_link_next", [])?;
        set_meta(&tx, "stats_dirty", 0)?;
        set_meta(&tx, "gl_cursor", 1)?;
        tx.commit()?;
        cursor = 1;
    }
    while Instant::now() < deadline {
        let rows: Vec<(i64, Option<String>, String, String)> = {
            let mut st = db.prepare_cached(
                "SELECT l.rowid, s.home, l.ns, l.key FROM link l JOIN item s ON s.rowid = l.src
                 WHERE l.rowid >= ?1 AND l.kind = 0 AND s.ns = 0 AND s.deleted = 0 ORDER BY l.rowid LIMIT ?2",
            )?;
            let rows = st.query_map(params![cursor, GROUP_CHUNK], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let tx = db.unchecked_transaction()?;
        if rows.is_empty() {
            tx.execute("DELETE FROM group_link", [])?;
            tx.execute("INSERT INTO group_link(a, b, links) SELECT a, b, links FROM group_link_next", [])?;
            tx.execute("DELETE FROM group_link_next", [])?;
            set_meta(&tx, "gl_cursor", 0)?;
            set_meta(&tx, "gl_as_of", now_ms(None))?;
            tx.commit()?;
            return Ok(());
        }
        let mut acc: HashMap<(String, String), i64> = HashMap::new();
        for (_, home, ns, key) in &rows {
            if ns != Namespace::Doc.prefix() {
                continue;
            }
            let Some(from) = home.as_deref().and_then(|h| tree.group_of(h)) else { continue };
            let to_home: Option<Option<String>> = tx
                .prepare_cached("SELECT home FROM item WHERE ns = 0 AND title = ?1 AND deleted = 0 ORDER BY created LIMIT 1")?
                .query_row([key], |r| r.get(0))
                .optional()?;
            let Some(to) = to_home.flatten().as_deref().and_then(|h| tree.group_of(h)).map(str::to_string) else { continue };
            if from != to {
                *acc.entry((from.to_string(), to)).or_default() += 1;
            }
        }
        for ((a, b), n) in acc {
            tx.prepare_cached("INSERT INTO group_link_next(a, b, links) VALUES (?1, ?2, ?3) ON CONFLICT(a, b) DO UPDATE SET links = links + excluded.links")?
                .execute(params![a, b, n])?;
        }
        cursor = rows.last().map(|r| r.0 + 1).unwrap_or(cursor);
        set_meta(&tx, "gl_cursor", cursor)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil(0), (1970, 1, 1));
        assert_eq!(super::day_name(1_790_812_800_000 / super::DAY), "2026-10-01");
        assert_eq!(super::day_name(-1), "1969-12-31");
        assert_eq!(super::day_name(19_782), "2024-02-29");
    }
}
