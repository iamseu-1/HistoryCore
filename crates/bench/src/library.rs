use history_corpus::GenDoc;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 대문 · 통계가 한 번에 읽는 모음: 큰 갈래별 문서 수와 최근 바뀐 문서 10.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Aggregates {
    pub groups: Vec<(String, u64)>,
    pub recent: Vec<u32>,
}

/// 잴 서재 구현. P1의 기준 구현은 `naive::NaiveSqlite`, P2의 서재 엔진도 이 약속에 꽂는다.
pub trait Library {
    fn name(&self) -> &'static str;
    /// 빈 자리에서 모두 들인다. 앞서 남은 것(끊긴 측정)은 지우고 시작한다.
    fn load(&mut self, docs: &mut dyn Iterator<Item = GenDoc>) -> Result<()>;
    /// 제목이 prefix로 시작하는 일반 문서(제목 차례), 많아야 limit.
    fn find_title(&self, prefix: &str, limit: usize) -> Result<Vec<u32>>;
    /// 본문에 query가 든 문서, 많아야 limit.
    fn search_body(&self, query: &str, limit: usize) -> Result<Vec<u32>>;
    /// 문서 열기 — 화면에 그릴 것을 꺼낸다(기준 구현은 본문 글, 엔진은 그리기 모델).
    fn open(&self, id: u32) -> Result<Option<String>>;
    /// 한 편 저장(판 + 색인 + 그리기 모델).
    fn save(&mut self, id: u32, body: &str) -> Result<()>;
    /// 분류 폴더 하나에 바로 든 문서.
    fn category_children(&self, category: &str) -> Result<Vec<u32>>;
    fn home_aggregates(&self) -> Result<Aggregates>;
    /// 닫았다 다시 연다(앱 시작의 엔진 몫).
    fn reopen(&mut self) -> Result<()>;
    fn bytes_on_disk(&self) -> Result<u64>;
    /// 들이기를 나눈 몫(이름, ms) — 엔진은 서재 들이기와 색인 짓기.
    fn load_parts(&self) -> Vec<(&'static str, u64)> {
        Vec::new()
    }
    /// 저장 공간을 나눈 몫(이름, 바이트).
    fn disk_parts(&self) -> Vec<(&'static str, u64)> {
        Vec::new()
    }
}
