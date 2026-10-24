/// 자리점 — 47043 → 47,043.
String grouped(int n) {
  final s = n.abs().toString();
  final b = StringBuffer(n < 0 ? '-' : '');
  for (var i = 0; i < s.length; i++) {
    if (i > 0 && (s.length - i) % 3 == 0) b.write(',');
    b.write(s[i]);
  }
  return b.toString();
}

/// 끝까지 세지 않은 수(질의의 한도에 닿음)는 「100+」.
String countLabel(int n, {int? cap}) => cap != null && n >= cap ? '${grouped(n)}+' : grouped(n);

/// 2026년 10월 19일.
String koreanDate(DateTime d) => '${d.year}년 ${d.month}월 ${d.day}일';
