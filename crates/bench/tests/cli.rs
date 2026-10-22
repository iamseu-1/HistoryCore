use std::process::Command;

#[test]
fn runs_quick_and_writes_both_reports() {
    // 기본은 서재 엔진, --library naive는 P1 기준 구현.
    for (extra, name) in [(None, "engine"), (Some("naive"), "naive-sqlite")] {
        let work = std::env::temp_dir().join(format!("측정 명령 {name} {}", std::process::id()));
        let out = work.join("결과");
        let _ = std::fs::remove_dir_all(&work);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hc-bench"));
        cmd.args(["run", "--docs", "300", "--quick", "--device", "시험", "--dir"]).arg(work.join("자리")).arg("--out").arg(&out);
        if let Some(l) = extra {
            cmd.args(["--library", l]);
        }
        assert!(cmd.status().unwrap().success(), "{name}");
        let md = std::fs::read_to_string(out.join(format!("시험-{name}-300.md"))).unwrap();
        assert!(md.contains("## 예산표(설계 9-2)"));
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join(format!("시험-{name}-300.json"))).unwrap()).unwrap();
        assert_eq!(json["docs"], 300);
        std::fs::remove_dir_all(&work).unwrap();
    }
}

#[test]
fn bad_arguments_exit_nonzero_with_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_hc-bench")).arg("nope").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("쓰는 법"));
}
