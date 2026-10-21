use history_library::{normalize_title, Clock, DocId, Hlc, Namespace, RevId};
use std::sync::{Arc, Mutex};

#[test]
fn ids_round_trip_through_text_and_bytes() {
    let d = DocId::new();
    assert_eq!(DocId::parse(&d.to_string()), Some(d));
    assert_eq!(DocId::from_slice(d.as_bytes()), Some(d));
    assert_eq!(d.to_string().len(), 36);
    assert_eq!(d.0.get_version_num(), 7);
    assert_eq!(RevId::parse("UUID 아님"), None);
}

#[test]
fn hlc_packs_wall_time_and_counter() {
    let h = Hlc::new(1_700_000_000_000, 5);
    assert_eq!(h.wall_ms(), 1_700_000_000_000);
    assert_eq!(h.counter(), 5);
    assert!(Hlc::new(1, 0) > Hlc::new(0, u16::MAX));
}

#[test]
fn the_clock_never_goes_backwards() {
    // Review Focus 5 — 벽시계가 뒤로 가도(시간대 · 수동 조정) 판 차례는 앞으로만 간다.
    let walls = Arc::new(Mutex::new(vec![1_000u64, 1_000, 900, 2_000, 1_500]));
    let w = walls.clone();
    let mut clock = Clock::new(Hlc(0), Box::new(move || w.lock().unwrap().remove(0)));
    let seq: Vec<Hlc> = (0..5).map(|_| clock.now()).collect();
    assert_eq!(
        seq,
        vec![Hlc::new(1_000, 0), Hlc::new(1_000, 1), Hlc::new(1_000, 2), Hlc::new(2_000, 0), Hlc::new(2_000, 1)]
    );
    assert_eq!(clock.last(), Hlc::new(2_000, 1));
}

#[test]
fn a_full_counter_moves_on_to_the_next_millisecond() {
    let mut clock = Clock::new(Hlc::new(5, u16::MAX), Box::new(|| 5u64));
    assert_eq!(clock.now(), Hlc::new(6, 0));
}

#[test]
fn namespaces_have_stable_codes_prefixes_and_numbers() {
    for ns in Namespace::ALL {
        assert_eq!(Namespace::from_code(ns.code()), Some(ns));
        assert_eq!(Namespace::from_i64(ns as i64), Some(ns));
    }
    assert_eq!(Namespace::Category.prefix(), "분류");
    assert_eq!(Namespace::Doc.link_title("임진왜란"), "임진왜란");
    assert_eq!(Namespace::Template.link_title("정보 상자"), "틀:정보 상자");
    assert_eq!(Namespace::from_code("연표"), None);
    assert_eq!(Namespace::from_i64(9), None);
}

#[test]
fn titles_are_nfc_and_trimmed() {
    assert_eq!(normalize_title("  임진왜란 ").unwrap(), "임진왜란");
    assert_eq!(normalize_title("\u{1100}\u{1161}").unwrap(), "가"); // 풀어 쓴 한글 → 모아 쓴 한글
    assert_eq!(normalize_title("\u{F900}").unwrap(), "\u{8C48}"); // 호환 한자 → 통합 한자
    assert_eq!(normalize_title("鶴 𪄃 😀").unwrap(), "鶴 𪄃 😀");
    assert_eq!(normalize_title("1:1 회담").unwrap(), "1:1 회담");
    assert_eq!(normalize_title("대문").unwrap(), "대문");
}

#[test]
fn titles_that_would_break_links_are_refused() {
    let long = "가".repeat(201);
    for bad in ["", "   ", "[[임진왜란]]", "가|나", "가#나", "가\n나", ":분류", "분류:조선", "틀 : 정보 상자", "字:鶴", "연표:abc", long.as_str()] {
        let e = normalize_title(bad).err().unwrap_or_else(|| panic!("받아들임: {bad:?}"));
        assert_eq!(e.code(), "bad_title", "{bad:?}");
    }
}
