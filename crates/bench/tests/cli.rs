use std::process::Command;

#[test]
fn runs_quick_and_writes_both_reports() {
    let work = std::env::temp_dir().join(format!("측정 명령 {}", std::process::id()));
    let out = work.join("결과");
    let _ = std::fs::remove_dir_all(&work);
    let status = Command::new(env!("CARGO_BIN_EXE_hc-bench"))
        .args(["run", "--docs", "300", "--quick", "--device", "시험", "--dir"])
        .arg(work.join("자리"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    let md = std::fs::read_to_string(out.join("시험-naive-sqlite-300.md")).unwrap();
    assert!(md.contains("## 예산표(설계 9-2)"));
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join("시험-naive-sqlite-300.json")).unwrap()).unwrap();
    assert_eq!(json["docs"], 300);
    std::fs::remove_dir_all(&work).unwrap();
}

#[test]
fn bad_arguments_exit_nonzero_with_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_hc-bench")).arg("nope").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("쓰는 법"));
}
