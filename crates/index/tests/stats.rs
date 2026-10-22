//! 대문 모음 · 서재 통계(설계 5-5 「통계 모음」 · 8-2).
use history_index::{Engine, HomeOptions};
use history_library::Namespace;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

const DAY: u64 = 86_400_000;
/// 2026-10-01 00:00 UTC.
const OCT_1: u64 = 1_790_812_800_000;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("색인 통계 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 시험이 움직이는 벽시계.
fn clock(at: &Arc<AtomicU64>) -> history_library::WallClock {
    let at = at.clone();
    Box::new(move || at.load(Ordering::SeqCst))
}

fn opts(days: usize, today: u64) -> HomeOptions {
    HomeOptions { recent: 10, wanted: 10, days, tz_minutes: 0, today_ms: Some(today) }
}

fn settle(e: &mut Engine) {
    for _ in 0..50 {
        e.step(Duration::from_millis(200)).unwrap();
        if e.status().unwrap().stale == 0 && !e.stats_pending().unwrap() {
            return;
        }
    }
    panic!("통계를 다 세지 못함");
}

#[test]
fn home_shape_and_rhythm_has_every_day() {
    let app = temp("대문");
    let now = Arc::new(AtomicU64::new(OCT_1 + 10 * 3_600_000));
    let mut e = Engine::open_with_clock(&app, clock(&now)).unwrap();
    e.create(Namespace::Category, "조선", "", "").unwrap();
    let a = e.create(Namespace::Doc, "가", "[[분류:조선]] [[없는 문서]]", "").unwrap();
    now.store(OCT_1 + 3 * DAY + 5 * 3_600_000, Ordering::SeqCst);
    e.save(a.doc, a.rev, "[[분류:조선]] [[없는 문서]] 고침", "").unwrap();
    e.create(Namespace::Doc, "나", "분류 없음", "").unwrap();
    let h = e.home(&opts(7, OCT_1 + 3 * DAY + 12 * 3_600_000)).unwrap();
    assert_eq!(h.groups.iter().map(|g| (g.name.as_str(), g.total)).collect::<Vec<_>>(), [("조선", 1)]);
    assert_eq!(h.uncategorized, 1);
    assert_eq!(h.recent.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(), ["나", "가", "조선"]);
    assert_eq!(h.wanted.iter().map(|w| w.title.as_str()).collect::<Vec<_>>(), ["없는 문서"]);
    let days: Vec<(&str, u64)> = h.rhythm.iter().map(|d| (d.day.as_str(), d.edits)).collect();
    assert_eq!(
        days,
        [("2026-09-28", 0), ("2026-09-29", 0), ("2026-09-30", 0), ("2026-10-01", 2), ("2026-10-02", 0), ("2026-10-03", 0), ("2026-10-04", 2)]
    );
    assert_eq!(h.counts.of(Namespace::Doc), 2);
    // 시간대: UTC+9로 보면 10월 1일 10시(UTC)는 같은 날 19시 — 날은 그대로, 10월 4일 5시(UTC)도 14시.
    let k = e.home(&HomeOptions { tz_minutes: 540, ..opts(1, OCT_1 + 3 * DAY + 12 * 3_600_000) }).unwrap();
    assert_eq!(k.rhythm.iter().map(|d| (d.day.as_str(), d.edits)).collect::<Vec<_>>(), [("2026-10-04", 2)]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn monthly_new_and_daily_edits_count_revisions_once() {
    let app = temp("달 · 날");
    let now = Arc::new(AtomicU64::new(OCT_1 + 3_600_000));
    let mut e = Engine::open_with_clock(&app, clock(&now)).unwrap();
    let a = e.create(Namespace::Doc, "가", "1", "").unwrap();
    let s = e.save(a.doc, a.rev, "2", "").unwrap();
    let s = e.save(a.doc, s.rev, "3", "").unwrap();
    e.save(a.doc, s.rev, "4", "").unwrap();
    now.store(OCT_1 + 40 * DAY, Ordering::SeqCst); // 11월 10일
    e.create(Namespace::Doc, "나", "x", "").unwrap();
    let c = e.create(Namespace::Doc, "다", "y", "").unwrap();
    e.delete(c.doc, c.rev, "").unwrap(); // 지운 문서는 새 문서 수에서 빠진다
    let st = e.stats(0, Some(OCT_1 + 40 * DAY)).unwrap();
    let months: Vec<(&str, u64)> = st.monthly_new.iter().map(|m| (m.month.as_str(), m.docs)).collect();
    assert_eq!(months, [("2026-10", 1), ("2026-11", 1)]);
    let edits = |day: &str| st.daily_edits.iter().find(|d| d.day == day).map(|d| d.edits);
    assert_eq!(edits("2026-10-01"), Some(4));
    assert_eq!(edits("2026-11-10"), Some(3));
    assert_eq!(st.daily_edits.len(), 365);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn group_links_catch_up_in_steps() {
    let app = temp("갈래 링크");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Category, "가 갈래", "", "").unwrap();
    e.create(Namespace::Category, "가 아래", "[[분류:가 갈래]]", "").unwrap();
    e.create(Namespace::Category, "나 갈래", "", "").unwrap();
    e.create(Namespace::Doc, "갑", "[[분류:가 아래]] [[을]] [[병]] [[없음]]", "").unwrap();
    e.create(Namespace::Doc, "을", "[[분류:나 갈래]] [[갑]]", "").unwrap();
    e.create(Namespace::Doc, "병", "[[분류:가 갈래]]", "").unwrap();
    assert!(e.stats_pending().unwrap());
    settle(&mut e);
    let st = e.stats(0, None).unwrap();
    let gl: Vec<(&str, &str, u64)> = st.group_links.iter().map(|g| (g.from.as_str(), g.to.as_str(), g.links)).collect();
    assert_eq!(gl, [("가 갈래", "나 갈래", 1), ("나 갈래", "가 갈래", 1)]); // 같은 갈래 안 링크(갑 → 병)는 세지 않는다
    assert!(st.as_of > 0);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}

#[test]
fn sources_linked_docs_and_decades() {
    let app = temp("사료 · 해");
    let mut e = Engine::open(&app).unwrap();
    e.create(Namespace::Template, "정보", "@기간@", "").unwrap();
    e.create(Namespace::Doc, "임진왜란", "[include(틀:정보, 기간=1592~1598)]\n[* 『선조실록』][* 『난중일기』]", "").unwrap();
    e.create(Namespace::Doc, "정유재란", "[[임진왜란]]\n[include(틀:정보, 기간=1597)]\n[* 『선조실록』]", "").unwrap();
    e.create(Namespace::Doc, "이순신", "[[임진왜란]] [[정유재란]]", "").unwrap();
    let st = e.stats(0, None).unwrap();
    assert_eq!(st.top_sources.iter().map(|s| (s.source.as_str(), s.refs)).collect::<Vec<_>>(), [("선조실록", 2), ("난중일기", 1)]);
    assert_eq!(st.top_linked.iter().map(|t| (t.doc.title.as_str(), t.refs)).collect::<Vec<_>>(), [("임진왜란", 2), ("정유재란", 1)]);
    assert_eq!(st.decades.iter().map(|d| (d.decade, d.docs)).collect::<Vec<_>>(), [(1590, 2)]);
    drop(e);
    std::fs::remove_dir_all(&app).unwrap();
}
