use history_namumark::{render, MBlock, MInline, Model, NoTemplates, ObjectRef, RenderOptions, TemplateStatus, Templates, MAX_TEMPLATE_BYTES, MAX_TEMPLATE_DEPTH};
use std::cell::Cell;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// 시간 한계 — 디버그 짓기는 느리고 CI는 붐벼 넉넉히, 릴리스는 빡빡하게(CI가 릴리스로도 돌린다).
const LIMIT: Duration = if cfg!(debug_assertions) { Duration::from_secs(30) } else { Duration::from_secs(3) };

fn plain() -> RenderOptions {
    RenderOptions::default()
}

fn templates(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn template_block(m: &Model, i: usize) -> (&TemplateStatus, &Vec<MBlock>) {
    match &m.blocks[i] {
        MBlock::Template { status, blocks, .. } => (status, blocks),
        other => panic!("틀 자리가 아님: {other:?}"),
    }
}

/// 모델 속 틀 자리의 상태를 모두(깊이 먼저).
fn statuses(blocks: &[MBlock], out: &mut Vec<TemplateStatus>) {
    for b in blocks {
        if let MBlock::Template { status, blocks, .. } = b {
            out.push(*status);
            statuses(blocks, out);
        }
    }
}

#[test]
fn the_json_contract_for_a_small_document() {
    // P3 library_view가 읽는 모양(spec/namumark.md) — 바뀌면 이 검사와 문서를 함께 고친다.
    let src = "== 개요 ==\n'''鶴'''은 새[* 물새]\n[목차]";
    let m = render(src, &plain(), &NoTemplates);
    let len = src.encode_utf16().count();
    assert_eq!(
        serde_json::to_value(&m).unwrap(),
        json!({
            "version": 1,
            "redirect": null,
            "blocks": [
                {"t": "heading", "level": 2, "number": "1", "section": 0, "folded": false, "inl": [{"t": "text", "s": "개요"}]},
                {"t": "para", "inl": [
                    {"t": "style", "style": "bold", "inl": [{"t": "han", "s": "鶴"}]},
                    {"t": "text", "s": "은 새"},
                    {"t": "fn", "n": 1}
                ]},
                {"t": "toc"}
            ],
            "footnotes": [{"n": 1, "name": null, "inl": [{"t": "text", "s": "물새"}]}],
            "sections": [{"index": 0, "level": 2, "number": "1", "title": "개요", "start": 0, "end": len}],
            "categories": [],
            "template_categories": [],
            "templates": []
        })
    );
    let back: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(back, m);
}

#[test]
fn templates_are_filled_before_parsing_and_their_footnotes_join_the_documents() {
    let t = templates(&[("틀:해전 정보", "'''@이름@''' — @날짜=날짜 모름@[* 틀 각주] [[@이름@]]\n[[분류:해전]]")]);
    let m = render("앞[* 첫째]\n[include(틀:해전 정보, 이름=한산도)]\n뒤[* 셋째]", &plain(), &t);
    let (status, blocks) = template_block(&m, 1);
    assert_eq!(*status, TemplateStatus::Ok);
    let MBlock::Para { inl } = &blocks[0] else { panic!("{blocks:?}") };
    assert_eq!(inl[0], MInline::Style { style: history_namumark::MStyle::Bold, inl: vec![MInline::Text { s: "한산도".into() }] });
    assert!(inl.contains(&MInline::Text { s: " — 날짜 모름".into() }));
    assert!(inl.iter().any(|n| matches!(n, MInline::Link { target, .. } if target.key == "한산도")));
    assert_eq!(m.footnotes.iter().map(|f| f.n).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(m.footnotes[1].inl, vec![MInline::Text { s: "틀 각주".into() }]);
    assert_eq!(m.template_categories, ["해전"]);
    assert!(m.categories.is_empty());
    assert_eq!(m.templates, ["틀:해전 정보"]);
}

#[test]
fn a_missing_template_is_marked_and_still_listed() {
    let m = render("[include(틀:없는 틀)]", &plain(), &NoTemplates);
    assert_eq!(*template_block(&m, 0).0, TemplateStatus::Missing);
    assert_eq!(m.templates, ["틀:없는 틀"]); // 나중에 그 틀이 생기면 다시 그릴 문서(설계 5-5 「틀을 쓰는 곳」)
}

#[test]
fn template_cycles_stop_and_are_marked() {
    // Review Focus 3
    let t = templates(&[("틀:가", "가\n[include(틀:나)]"), ("틀:나", "나\n[include(틀:가)]"), ("틀:스스로", "[include(틀:스스로)]")]);
    let m = render("[include(틀:가)]\n[include(틀:스스로)]", &plain(), &t);
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s, [TemplateStatus::Ok, TemplateStatus::Ok, TemplateStatus::Cycle, TemplateStatus::Ok, TemplateStatus::Cycle]);
}

#[test]
fn template_chains_stop_at_the_depth_limit() {
    let pairs: Vec<(String, String)> = (0..12).map(|k| (format!("틀:{k}"), format!("{k}단\n[include(틀:{})]", k + 1))).collect();
    let t: BTreeMap<String, String> = pairs.into_iter().collect();
    let m = render("[include(틀:0)]", &plain(), &t);
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s.iter().filter(|x| **x == TemplateStatus::Ok).count(), MAX_TEMPLATE_DEPTH);
    assert_eq!(s.last(), Some(&TemplateStatus::TooDeep));
}

#[test]
fn template_explosions_stop_quickly() {
    // 틀 하나가 다른 틀을 50번, 그 틀이 또 50번 — 2,550번 펼칠 것을 2,000번에서 멈춘다.
    let inner = "[include(틀:잎)]\n".repeat(50);
    let t = templates(&[("틀:가지", inner.as_str()), ("틀:잎", "잎")]);
    let doc = "[include(틀:가지)]\n".repeat(50);
    let started = Instant::now();
    let m = render(&doc, &plain(), &t);
    assert!(started.elapsed() < LIMIT);
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s.iter().filter(|x| **x == TemplateStatus::Ok).count(), 2_000);
    assert!(s.contains(&TemplateStatus::TooMany));
}

#[test]
fn a_renamed_template_is_followed_once() {
    // P2-1의 이름 바꾸기는 옛 제목에 `#넘겨주기 틀:새 이름`을 남긴다.
    let t = templates(&[("틀:옛 이름", "#넘겨주기 틀:새 이름\n"), ("틀:새 이름", "새 틀 본문"), ("틀:두 번", "#넘겨주기 틀:옛 이름")]);
    let m = render("[include(틀:옛 이름)]\n[include(틀:두 번)]", &plain(), &t);
    let (status, blocks) = template_block(&m, 0);
    assert_eq!(*status, TemplateStatus::Ok);
    assert_eq!(blocks, &vec![MBlock::Para { inl: vec![MInline::Text { s: "새 틀 본문".into() }] }]);
    assert_eq!(*template_block(&m, 1).0, TemplateStatus::Missing); // 넘겨주기를 두 번은 따르지 않는다
    assert_eq!(m.templates, ["틀:옛 이름", "틀:새 이름", "틀:두 번"]);
}

#[test]
fn section_ranges_are_utf16_so_dart_cuts_the_same_text() {
    // Review Focus 4 — 앞에 𪄃(UTF-16 두 칸) · 한글 · 😀가 있어도 Dart 쪽에서 같은 문단이 잘린다.
    let src = "𪄃 머리말\n== 鶴 ==\n본문 😀\n== 끝 ==\n끝";
    let m = render(src, &plain(), &NoTemplates);
    let units: Vec<u16> = src.encode_utf16().collect();
    let cut = |i: usize| String::from_utf16(&units[m.sections[i].start..m.sections[i].end]).unwrap();
    assert_eq!(cut(0), "== 鶴 ==\n본문 😀\n");
    assert_eq!(cut(1), "== 끝 ==\n끝");
}

#[test]
fn hanja_runs_are_their_own_spans() {
    let m = render("鶴𪄃은 漢字, [ruby(漢字, ruby=한자)]", &plain(), &NoTemplates);
    let MBlock::Para { inl } = &m.blocks[0] else { panic!() };
    assert_eq!(inl[0], MInline::Han { s: "鶴𪄃".into() });
    assert_eq!(inl[1], MInline::Text { s: "은 ".into() });
    assert_eq!(inl[2], MInline::Han { s: "漢字".into() });
    assert!(matches!(&inl[4], MInline::Ruby { base, .. } if base == "漢字"));
}

#[test]
fn redirect_documents_have_no_body() {
    let m = render("#넘겨주기 임진왜란", &plain(), &NoTemplates);
    assert_eq!(m.redirect.as_ref().map(|r| r.key.as_str()), Some("임진왜란"));
    assert!(m.blocks.is_empty());
}

#[test]
fn a_template_page_shows_its_parameters_and_cannot_include_itself() {
    let t = templates(&[("틀:해전 정보", "@이름=미상@\n[include(틀:해전 정보)]")]);
    let m = render("@이름=미상@\n[include(틀:해전 정보)]", &RenderOptions { title: Some("해전 정보".into()), template: true }, &t);
    let MBlock::Para { inl } = &m.blocks[0] else { panic!("{:?}", m.blocks) };
    assert!(matches!(&inl[0], MInline::Param { name, fallback: Some(f) } if name == "이름" && f == "미상"));
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s, [TemplateStatus::Cycle]);
}

#[test]
fn template_text_that_multiplies_stops_at_the_size_limit() {
    // 최종 검토 2 — 인자를 다섯 번씩 넘기는 틀 여덟 단: 30바이트 인자가 일곱째 단에서 30 × 5^8 ≈ 11.7MB가 된다.
    let mut t = BTreeMap::new();
    for d in 0..8 {
        t.insert(format!("틀:T{d}"), format!("[include(틀:T{}, a={})]", d + 1, "@a@".repeat(5)));
    }
    t.insert("틀:T8".to_string(), "@a@".to_string());
    let started = Instant::now();
    let m = render(&format!("[include(틀:T0, a={})]", "가".repeat(10)), &plain(), &t);
    assert!(started.elapsed() < LIMIT, "{:?}", started.elapsed());
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s.last(), Some(&TemplateStatus::TooLarge), "{s:?}");
    assert!(serde_json::to_string(&m).unwrap().len() < MAX_TEMPLATE_BYTES);
}

#[test]
fn a_template_over_the_size_limit_is_refused_quickly_and_read_once() {
    struct Counting(Cell<usize>, String);
    impl Templates for Counting {
        fn source(&self, _: &ObjectRef) -> Option<String> {
            self.0.set(self.0.get() + 1);
            Some(self.1.clone())
        }
    }
    let big = Counting(Cell::new(0), "가".repeat(MAX_TEMPLATE_BYTES / 3 + 1));
    let started = Instant::now();
    let m = render(&"[include(틀:큰 틀)]\n".repeat(2_000), &plain(), &big);
    assert!(started.elapsed() < LIMIT, "{:?}", started.elapsed());
    let mut s = Vec::new();
    statuses(&m.blocks, &mut s);
    assert_eq!(s.len(), 2_000);
    assert!(s.iter().all(|x| *x == TemplateStatus::TooLarge));
    assert_eq!(big.0.get(), 1); // 한 번 그리는 동안 같은 틀은 한 번만 읽는다
}
