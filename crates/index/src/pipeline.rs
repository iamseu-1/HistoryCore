//! 여러 스레드로 일하고 한 곳에서 들어온 차례대로 받는다 — 통째 짓기에서 해석 · 그리기 · 조각내기는 일꾼이, 쓰기는 하나가 한다.
use std::collections::BTreeMap;
use std::sync::mpsc::sync_channel;
use std::sync::Mutex;

/// `items`를 `workers`개 스레드에서 `work`로 바꾸고, 들어온 차례대로 `sink`에 넘긴다.
/// `sink`가 Err를 내면 일꾼을 멈추고 그 Err를 돌려준다.
pub fn ordered<T, R, E>(
    items: impl Iterator<Item = T> + Send,
    workers: usize,
    work: impl Fn(T) -> R + Sync,
    mut sink: impl FnMut(R) -> Result<(), E>,
) -> Result<(), E>
where
    T: Send,
    R: Send,
{
    let workers = workers.max(1);
    let source = Mutex::new(items.enumerate());
    let (tx, rx) = sync_channel::<(usize, R)>(workers * 64);
    std::thread::scope(|s| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (source, work) = (&source, &work);
            s.spawn(move || loop {
                let next = source.lock().unwrap_or_else(|e| e.into_inner()).next();
                let Some((i, item)) = next else { break };
                if tx.send((i, work(item))).is_err() {
                    break; // 받는 쪽이 멈췄다
                }
            });
        }
        drop(tx);
        // 받는 끝을 이 안에서 갖고 있어야, 일찍 돌아갈 때 끝이 닫혀 일꾼이 보내기에서 풀려난다.
        let rx = rx;
        let mut waiting = BTreeMap::new();
        let mut want = 0usize;
        for (i, r) in rx.iter() {
            waiting.insert(i, r);
            while let Some(r) = waiting.remove(&want) {
                sink(r)?;
                want += 1;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order_and_stops_on_error() {
        let mut got = Vec::new();
        super::ordered(0..10_000u32, 8, |x| x * 2, |y| {
            got.push(y);
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(got, (0..10_000u32).map(|x| x * 2).collect::<Vec<_>>());
        let mut n = 0;
        let r = super::ordered(0..1_000_000u32, 8, |x| x, |_| {
            n += 1;
            if n == 100 {
                Err("멈춤")
            } else {
                Ok(())
            }
        });
        assert_eq!((r, n), (Err("멈춤"), 100));
    }
}
