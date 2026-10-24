//! 동기화 — 묶음 내보내기 · 적용 · 보류 · 시계 · 자동 합치기 · 충돌 · 합류 · 내보내기(spec/drive-format.md).
use history_library::{Doc, DocId, Error, Library, Namespace, RevId};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("서재 동기화 {name} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 같은 서재의 기기 둘 — 둘째는 첫째 서재에 합류한다.
fn pair(name: &str) -> (Library, Library, PathBuf, PathBuf) {
    let (a_dir, b_dir) = (temp(&format!("{name} 가")), temp(&format!("{name} 나")));
    let a = Library::open(&a_dir).unwrap();
    let mut b = Library::open(&b_dir).unwrap();
    b.join(a.info().library).unwrap();
    (a, b, a_dir, b_dir)
}

/// `from`이 `after` 뒤에 만든 판을 묶음 하나로 — 없으면 None.
fn pack(from: &Library, after: i64) -> Option<Vec<u8>> {
    from.export(after, 1).unwrap().map(|e| e.pack)
}

/// 서로 맞춘다 — 양쪽이 지금까지 만든 판을 모두 주고받는다.
fn exchange(a: &mut Library, b: &mut Library) {
    let (pa, pb) = (pack(a, 0), pack(b, 0));
    if let Some(p) = pb {
        a.apply(&p).unwrap();
    }
    if let Some(p) = pa {
        b.apply(&p).unwrap();
    }
}

fn get(lib: &Library, doc: DocId) -> Doc {
    lib.get(doc).unwrap().unwrap()
}

#[test]
fn export_carries_only_this_devices_revisions() {
    let (mut a, mut b, ..) = pair("내 판만");
    a.create(Namespace::Doc, "임진왜란", "첫 판", "").unwrap();
    a.create(Namespace::Category, "조선", "[[분류:한국사]]", "").unwrap();
    let e = a.export(0, 1).unwrap().unwrap();
    assert_eq!((e.count, e.seq, e.more), (2, 2, false));
    let r = b.apply(&e.pack).unwrap();
    assert_eq!((r.added, r.skipped, r.held), (2, 0, 0));
    // 받은 판은 내보내지 않고 쌓인 수에도 세지 않는다 — 번호(seq)는 오른다.
    assert!(b.export(0, 1).unwrap().is_none());
    let s = b.sync_status(0).unwrap();
    assert_eq!((s.seq, s.pending, s.held, s.conflicts), (2, 0, 0, 0));
    b.create(Namespace::Doc, "병자호란", "나의 판", "").unwrap();
    assert_eq!(b.sync_status(0).unwrap().pending, 1);
    assert_eq!(b.export(0, 1).unwrap().unwrap().count, 1);
}

#[test]
fn the_same_pack_twice_goes_in_once() {
    let (mut a, mut b, ..) = pair("두 번");
    let s = a.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
    let p = pack(&a, 0).unwrap();
    assert_eq!(b.apply(&p).unwrap().added, 1);
    let again = b.apply(&p).unwrap();
    assert_eq!((again.added, again.skipped), (0, 1));
    assert_eq!(b.counts().unwrap().total(), 1);
    assert_eq!(get(&b, s.doc).body, "본문");
    assert_eq!(b.history(s.doc, 10, None).unwrap().len(), 1);
}

#[test]
fn a_revision_waits_for_its_parent_even_across_reopen() {
    let (mut a, mut b, _, b_dir) = pair("보류");
    let s = a.create(Namespace::Doc, "임진왜란", "첫 판", "").unwrap();
    let first = a.export(0, 1).unwrap().unwrap();
    a.save(s.doc, s.rev, "둘째 판", "").unwrap();
    let second = a.export(first.seq, 2).unwrap().unwrap();
    let r = b.apply(&second.pack).unwrap();
    assert_eq!((r.added, r.held), (0, 1));
    assert!(b.get(s.doc).unwrap().is_none());
    assert_eq!(b.sync_status(0).unwrap().held, 1);
    drop(b);
    let mut b = Library::open(&b_dir).unwrap();
    assert_eq!(b.sync_status(0).unwrap().held, 1, "보류 판은 다시 열어도 남는다");
    let r = b.apply(&first.pack).unwrap();
    assert_eq!((r.added, r.held), (2, 0));
    assert_eq!(get(&b, s.doc).body, "둘째 판");
}

#[test]
fn a_pack_from_another_library_changes_nothing() {
    let (mut a, ..) = pair("다른 서재");
    let mut other = Library::open(&temp("다른 서재 셋")).unwrap();
    a.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
    let p = pack(&a, 0).unwrap();
    assert!(matches!(other.apply(&p), Err(Error::WrongLibrary(_))));
    assert_eq!(other.counts().unwrap().total(), 0);
    assert!(matches!(other.apply("zstd 아님".as_bytes()), Err(Error::BadPack(_))));
}

#[test]
fn a_big_backlog_goes_out_in_several_packs() {
    let (mut a, mut b, ..) = pair("나눠 내기");
    let docs = (0..1_200).map(|i| history_library::NewDoc { ns: Namespace::Doc, title: format!("문서 {i}"), body: format!("{i}번") });
    a.import(&mut docs.into_iter(), 300).unwrap();
    let (mut after, mut no, mut sizes) = (0, 1, Vec::new());
    while let Some(e) = a.export(after, no).unwrap() {
        sizes.push(e.count);
        assert_eq!(e.more, after + (e.count as i64) < 1_200);
        b.apply(&e.pack).unwrap();
        after = e.seq;
        no += 1;
    }
    assert_eq!(sizes, vec![500, 500, 200]);
    assert_eq!(b.counts().unwrap().total(), 1_200);
}

#[test]
fn the_clock_moves_past_what_it_received() {
    let a_dir = temp("시계 가");
    let mut a = Library::open(&a_dir).unwrap();
    // 나 기기의 벽시계는 1970년에 멈춰 있다.
    let mut b = Library::open_with_clock(&temp("시계 나"), Box::new(|| 1_000)).unwrap();
    b.join(a.info().library).unwrap();
    let s = a.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
    b.apply(&pack(&a, 0).unwrap()).unwrap();
    let mine = b.save(s.doc, s.rev, "고침", "").unwrap();
    assert!(mine.hlc > s.hlc, "받은 뒤 고친 판이 늦다");
    assert_eq!(get(&b, s.doc).head, mine.rev);
}

#[test]
fn edits_on_different_lines_merge_the_same_way_on_both_devices() {
    let (mut a, mut b, ..) = pair("자동 합침");
    let s = a.create(Namespace::Doc, "임진왜란", "첫 줄\n둘째 줄\n셋째 줄\n", "").unwrap();
    exchange(&mut a, &mut b);
    a.save(s.doc, s.rev, "가가 고친 첫 줄\n둘째 줄\n셋째 줄\n", "").unwrap();
    b.save(s.doc, s.rev, "첫 줄\n둘째 줄\n나가 고친 셋째 줄\n", "").unwrap();
    let (pa, pb) = (a.export(1, 2).unwrap().unwrap(), b.export(0, 1).unwrap().unwrap());
    let ra = a.apply(&pb.pack).unwrap();
    let rb = b.apply(&pa.pack).unwrap();
    assert_eq!((ra.forked, ra.merged, rb.merged), (1, 1, 1));
    assert!(ra.conflicts.is_empty() && rb.conflicts.is_empty());
    let (da, db) = (get(&a, s.doc), get(&b, s.doc));
    assert_eq!(da.body, "가가 고친 첫 줄\n둘째 줄\n나가 고친 셋째 줄\n");
    assert_eq!((da.head, &da.body, da.updated), (db.head, &db.body, db.updated), "합친 판은 결정적");
    let merged = a.revision(da.head).unwrap().unwrap();
    assert_eq!(merged.info.message, "자동 합침");
    assert!(merged.info.parent.is_some() && merged.info.parent2.is_some());
    assert!(a.conflict_leaves(s.doc).unwrap().is_empty());
    // 합친 판은 두 기기 모두 「내 판」으로 내보내지만 받는 쪽은 ID로 건너뛴다.
    let again = a.apply(&b.export(pb.seq, 2).unwrap().unwrap().pack).unwrap();
    assert_eq!((again.added, again.skipped), (0, 1));
}

#[test]
fn the_same_line_changed_twice_is_a_conflict_until_merged_by_hand() {
    let (mut a, mut b, ..) = pair("충돌");
    let s = a.create(Namespace::Doc, "임진왜란", "첫 줄\n둘째 줄\n", "").unwrap();
    exchange(&mut a, &mut b);
    let sa = a.save(s.doc, s.rev, "첫 줄\n가의 둘째 줄\n", "").unwrap();
    let sb = b.save(s.doc, s.rev, "첫 줄\n나의 둘째 줄\n", "").unwrap();
    let ra = a.apply(&b.export(0, 1).unwrap().unwrap().pack).unwrap();
    let rb = b.apply(&a.export(1, 2).unwrap().unwrap().pack).unwrap();
    assert_eq!((ra.conflicts.clone(), rb.conflicts.clone()), (vec![s.doc], vec![s.doc]));
    assert_eq!(ra.merged, 0);
    let later = if (sb.hlc, sb.rev) > (sa.hlc, sa.rev) { sb.rev } else { sa.rev };
    let earlier = if later == sa.rev { sb.rev } else { sa.rev };
    assert_eq!((get(&a, s.doc).head, get(&b, s.doc).head), (later, later), "머리는 늦은 판 — 두 기기가 같다");
    assert_eq!(a.conflict_leaves(s.doc).unwrap(), vec![earlier]);
    assert_eq!(a.sync_status(0).unwrap().conflicts, 1);
    // 견주어 합친다 — 앞 판이 둘인 판이 생기고 충돌이 풀린다. 나도 받으면 풀린다.
    let after = a.sync_status(0).unwrap().seq;
    let m = a.save_merge(s.doc, later, earlier, "첫 줄\n둘이 합친 둘째 줄\n", "견주어 합침").unwrap();
    assert!(a.conflict_leaves(s.doc).unwrap().is_empty());
    assert_eq!(a.sync_status(0).unwrap().conflicts, 0);
    let info = a.revision(m.rev).unwrap().unwrap().info;
    assert_eq!((info.parent, info.parent2), (Some(later), Some(earlier)));
    b.apply(&a.export(after, 3).unwrap().unwrap().pack).unwrap();
    assert_eq!(b.sync_status(0).unwrap().conflicts, 0);
    assert_eq!(get(&b, s.doc).body, "첫 줄\n둘이 합친 둘째 줄\n");
    // 머리가 아닌 판이나 잎이 아닌 판으로는 합칠 수 없다.
    assert!(matches!(a.save_merge(s.doc, m.rev, s.rev, "x", ""), Err(Error::NotALeaf(_))));
}

#[test]
fn a_rename_and_an_edit_merge_into_both() {
    let (mut a, mut b, ..) = pair("이름과 본문");
    let s = a.create(Namespace::Doc, "임진 왜란", "본문\n", "").unwrap();
    exchange(&mut a, &mut b);
    a.rename(s.doc, s.rev, "임진왜란", false, "").unwrap();
    b.save(s.doc, s.rev, "고친 본문\n", "").unwrap();
    exchange(&mut a, &mut b);
    for lib in [&a, &b] {
        let d = get(lib, s.doc);
        assert_eq!((d.title.as_str(), d.body.as_str()), ("임진왜란", "고친 본문\n"));
    }
    assert_eq!(a.by_title(Namespace::Doc, "임진왜란").unwrap().len(), 1);
}

#[test]
fn two_different_renames_conflict() {
    let (mut a, mut b, ..) = pair("이름 둘");
    let s = a.create(Namespace::Doc, "임진 왜란", "본문\n", "").unwrap();
    exchange(&mut a, &mut b);
    a.rename(s.doc, s.rev, "임진왜란", false, "").unwrap();
    b.rename(s.doc, s.rev, "壬辰倭亂", false, "").unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.conflict_leaves(s.doc).unwrap().len(), 1);
    assert_eq!(get(&a, s.doc).title, get(&b, s.doc).title);
}

#[test]
fn an_edit_beats_a_delete() {
    let (mut a, mut b, ..) = pair("고침이 이김");
    let s = a.create(Namespace::Doc, "임진왜란", "본문", "").unwrap();
    exchange(&mut a, &mut b);
    a.delete(s.doc, s.rev, "").unwrap();
    b.save(s.doc, s.rev, "고친 본문", "").unwrap();
    let ra = a.apply(&b.export(0, 1).unwrap().unwrap().pack).unwrap();
    assert_eq!(ra.revived, vec![s.doc]);
    b.apply(&a.export(1, 2).unwrap().unwrap().pack).unwrap();
    for lib in [&a, &b] {
        let d = get(lib, s.doc);
        assert!(!d.deleted);
        assert_eq!(d.body, "고친 본문");
        assert!(lib.conflict_leaves(s.doc).unwrap().is_empty(), "지움 잎은 충돌이 아니다");
    }
    assert_eq!(a.counts().unwrap().deleted, 0);
}

#[test]
fn joining_keeps_this_devices_documents_and_sends_them() {
    let a_dir = temp("합류 가");
    let mut a = Library::open(&a_dir).unwrap();
    let b_dir = temp("합류 나");
    let mut b = Library::open(&b_dir).unwrap();
    a.create(Namespace::Doc, "임진왜란", "가의 문서", "").unwrap();
    b.create(Namespace::Doc, "임진왜란", "나의 문서", "").unwrap();
    assert!(matches!(a.apply(&pack(&b, 0).unwrap()), Err(Error::WrongLibrary(_))));
    b.join(a.info().library).unwrap();
    assert_eq!(b.info().library, a.info().library);
    drop(b);
    let mut b = Library::open(&b_dir).unwrap();
    assert_eq!(b.info().library, a.info().library, "합류는 다시 열어도 남는다");
    exchange(&mut a, &mut b);
    // 같은 제목 문서 둘 — 둘 다 남는다(설계 6-4).
    assert_eq!(a.by_title(Namespace::Doc, "임진왜란").unwrap().len(), 2);
    assert_eq!(b.by_title(Namespace::Doc, "임진왜란").unwrap().len(), 2);
}

#[test]
fn a_backup_is_a_zip_with_a_copy_of_the_library() {
    let (mut a, ..) = pair("내보내기");
    for i in 0..30 {
        a.create(Namespace::Doc, &format!("문서 {i}"), "본문", "").unwrap();
    }
    let out = temp("내보내기 파일").join("서재 내보내기.zip");
    a.backup(&out).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let names: BTreeSet<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
    assert_eq!(names, BTreeSet::from(["library.db".to_string(), "읽어보기.txt".to_string()]));
    let dir = temp("내보내기 풀기");
    std::fs::create_dir_all(dir.join(history_library::FOLDER)).unwrap();
    let mut f = std::fs::File::create(dir.join(history_library::FOLDER).join(history_library::FILE)).unwrap();
    std::io::copy(&mut zip.by_name("library.db").unwrap(), &mut f).unwrap();
    drop(f);
    let copy = Library::open(&dir).unwrap();
    assert_eq!(copy.counts().unwrap().total(), 30);
    assert_eq!(copy.info().library, a.info().library);
}

// ── 수렴 성질 검사(설계 9-3) ──

/// 작고 결정적인 난수(SplitMix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// 기기 하나 — 서재와 지금까지 올린 묶음.
struct Device {
    lib: Library,
    sent: i64,
    packs: Vec<Vec<u8>>,
}

impl Device {
    /// 쌓인 판을 묶음으로 「올린다」.
    fn push(&mut self) {
        while let Some(e) = self.lib.export(self.sent, self.packs.len() as u64 + 1).unwrap() {
            self.sent = e.seq;
            self.packs.push(e.pack);
        }
    }
}

fn edit(rng: &mut Rng, body: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
    let word = format!("말{}", rng.below(1000));
    match rng.below(3) {
        0 if !lines.is_empty() => {
            let i = rng.below(lines.len());
            lines[i] = word;
        }
        1 if lines.len() > 1 => {
            lines.remove(rng.below(lines.len()));
        }
        _ => {
            let i = rng.below(lines.len() + 1);
            lines.insert(i, word);
        }
    }
    lines.iter().map(|l| format!("{l}\n")).collect()
}

/// 기기 하나에서 무작위 일 하나.
fn act(rng: &mut Rng, d: &mut Device) {
    let docs = d.lib.doc_ids().unwrap();
    if docs.is_empty() || rng.chance(15) {
        let n = rng.below(40);
        d.lib.create(Namespace::Doc, &format!("문서 {n}"), "하나\n둘\n셋\n넷\n다섯\n", "").unwrap();
        return;
    }
    let doc = get(&d.lib, docs[rng.below(docs.len())]);
    let leaves = d.lib.conflict_leaves(doc.id).unwrap();
    match rng.below(20) {
        0 if !doc.deleted => {
            d.lib.delete(doc.id, doc.head, "").unwrap();
        }
        1 if !doc.deleted => {
            let _ = d.lib.rename(doc.id, doc.head, &format!("이름 {}", rng.below(30)), rng.chance(50), "");
        }
        2 | 3 if !leaves.is_empty() => {
            d.lib.save_merge(doc.id, doc.head, leaves[0], &edit(rng, &doc.body), "견주어 합침").unwrap();
        }
        _ => {
            d.lib.save(doc.id, doc.head, &edit(rng, &doc.body), "").unwrap();
        }
    }
}

/// 서재의 모든 판(기기 열을 뺀 것) · 문서 머리 · 충돌 — 기기끼리 같아야 한다.
type Shape = (BTreeMap<RevId, String>, BTreeMap<DocId, (RevId, bool, String)>, BTreeSet<DocId>);

fn shape(lib: &Library) -> Shape {
    let (mut revs, mut heads, mut conflicts) = (BTreeMap::new(), BTreeMap::new(), BTreeSet::new());
    for doc in lib.doc_ids().unwrap() {
        let d = get(lib, doc);
        heads.insert(doc, (d.head, d.deleted, d.title.clone()));
        if !lib.conflict_leaves(doc).unwrap().is_empty() {
            conflicts.insert(doc);
        }
        for r in lib.history(doc, 10_000, None).unwrap() {
            let body = lib.revision(r.id).unwrap().unwrap().body;
            let row = format!("{:?}|{:?}|{:?}|{}|{:?}|{:?}|{}|{}|{body}", r.doc, r.parent, r.parent2, r.hlc.0, r.kind, r.ns, r.title, r.message);
            revs.insert(r.id, row);
        }
    }
    (revs, heads, conflicts)
}

fn converge(seed: u64) -> (usize, usize) {
    let mut rng = Rng(seed);
    let first = Library::open(&temp(&format!("수렴 {seed} 0"))).unwrap();
    let library = first.info().library;
    let mut devices = vec![Device { lib: first, sent: 0, packs: Vec::new() }];
    for i in 1..3 {
        let mut lib = Library::open(&temp(&format!("수렴 {seed} {i}"))).unwrap();
        lib.join(library).unwrap();
        devices.push(Device { lib, sent: 0, packs: Vec::new() });
    }
    for _ in 0..200 {
        let i = rng.below(3);
        if rng.chance(70) {
            act(&mut rng, &mut devices[i]);
        } else if rng.chance(50) {
            devices[i].push();
        } else {
            // 다른 기기의 묶음 하나를 아무 차례로 받는다(겹쳐 받기 · 빠뜨렸다 나중에 받기 포함).
            let j = (i + 1 + rng.below(2)) % 3;
            if !devices[j].packs.is_empty() {
                let p = devices[j].packs[rng.below(devices[j].packs.len())].clone();
                devices[i].lib.apply(&p).unwrap();
            }
        }
    }
    // 끝 — 더 올릴 판이 없을 때까지 모두 올리고 모두 받는다(합친 판도 새 판이다).
    for round in 0.. {
        assert!(round < 10, "씨앗 {seed}: 열 번 맞춰도 조용해지지 않음");
        let before: Vec<usize> = devices.iter().map(|d| d.packs.len()).collect();
        for d in devices.iter_mut() {
            d.push();
        }
        let all: Vec<Vec<Vec<u8>>> = devices.iter().map(|d| d.packs.clone()).collect();
        for (i, d) in devices.iter_mut().enumerate() {
            for (j, packs) in all.iter().enumerate() {
                if i != j {
                    for p in packs {
                        d.lib.apply(p).unwrap();
                    }
                }
            }
        }
        for d in devices.iter_mut() {
            d.push();
        }
        if devices.iter().map(|d| d.packs.len()).collect::<Vec<_>>() == before && round > 0 {
            break;
        }
    }
    let shapes: Vec<Shape> = devices.iter().map(|d| shape(&d.lib)).collect();
    assert!(!shapes[0].0.is_empty());
    for (i, s) in shapes.iter().enumerate().skip(1) {
        assert_eq!(s.0, shapes[0].0, "씨앗 {seed}: 기기 {i}의 판 집합");
        assert_eq!(s.1, shapes[0].1, "씨앗 {seed}: 기기 {i}의 머리");
        assert_eq!(s.2, shapes[0].2, "씨앗 {seed}: 기기 {i}의 충돌");
    }
    for d in &devices {
        let s = d.lib.sync_status(d.sent).unwrap();
        assert_eq!((s.pending, s.held), (0, 0), "씨앗 {seed}: 끝에 쌓인 판 · 보류 판 없음");
    }
    let merges = shapes[0].0.values().filter(|row| row.contains("|자동 합침|")).count();
    (merges, shapes[0].2.len())
}

#[test]
fn three_devices_end_up_with_the_same_library() {
    let (mut merges, mut conflicts) = (0, 0);
    for seed in 1..=12 {
        let (m, c) = converge(seed);
        merges += m;
        conflicts += c;
    }
    // 검사가 헛돌지 않게 — 씨앗들 가운데 자동 합침과 남은 충돌이 실제로 있었다.
    eprintln!("자동 합침 {merges} · 남은 충돌 {conflicts}");
    assert!(merges > 0 && conflicts > 0, "자동 합침 {merges} · 남은 충돌 {conflicts}");
}
