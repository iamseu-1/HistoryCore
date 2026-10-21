"""《康熙字典》 한국어 번역층에서 더미 서재의 재료(문장 · 낱말)를 뽑는다. 한 번 돌려 결과를 커밋한다.

  py -3 -I -X utf8 tools/make-pool.py <번역층 폴더> crates/corpus/assets
  예) py -3 -I -X utf8 tools/make-pool.py "D:/HistoryArchive/data/kangxi/translations/ko" crates/corpus/assets

늘 같은 결과가 나오도록 해시 차례로 고른다. 나무마크에서 뜻이 있는 글자(대괄호 · | · = · * · 중괄호 · ~ · ^ · _ · ' · -- · ,,)가 든 문장은 뺀다.
"""
import hashlib
import io
import json
import re
import sys
from collections import Counter
from pathlib import Path

SRC, OUT = Path(sys.argv[1]), Path(sys.argv[2])
SENTENCES, WORDS = 20000, 5000
BAD = re.compile(r"[\[\]|=*⟦⟧{}~^_'\n\r\t]|--|,,")
SUP = re.compile(r'[¹²³⁴⁵⁶⁷⁸⁹⁰]+')
STOP = {'이다', '하였다', '라고', '하는', '있다', '것이다', '같다', '이며', '으로', '에서', '모두',
        '또한', '이라', '하고', '한다', '되었다', '그리고', '이니', '이고', '것이', '한다고'}

sents, words = set(), Counter()
for p in sorted(SRC.glob('*.jsonl')):
    for line in io.open(p, encoding='utf-8'):
        for para in json.loads(line).get('paragraphs', []):
            ko = para.get('ko')
            if not ko:
                continue
            for s in re.split(r'(?<=다\.)\s+', SUP.sub('', ko)):
                s = s.strip()
                if 15 <= len(s) <= 160 and s.endswith('다.') and not BAD.search(s):
                    sents.add(s)
                    for w in re.findall(r'[가-힣]{2,4}', s):
                        if w not in STOP:
                            words[w] += 1

key = lambda s: hashlib.sha256(s.encode('utf-8')).hexdigest()
picked = sorted(sorted(sents, key=key)[:SENTENCES])
top = [w for w, _ in sorted(words.items(), key=lambda kv: (-kv[1], kv[0]))][:WORDS]
OUT.mkdir(parents=True, exist_ok=True)
(OUT / 'pool.txt').write_text('\n'.join(picked) + '\n', encoding='utf-8', newline='\n')
(OUT / 'words.txt').write_text('\n'.join(top) + '\n', encoding='utf-8', newline='\n')
print(f'문장 {len(picked)} / 후보 {len(sents)} · 낱말 {len(top)}')
