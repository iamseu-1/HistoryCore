import 'package:flutter/widgets.dart';

/// 쌍둥이 HTML(검사 틀)에서 이 위젯이 될 요소 이름. 화면에는 아무 영향이 없다 — 렌더 트리에 이름만 남긴다.
/// 랜드마크(`aside` · `nav` · `main`)와 표(`table` · `tr` · `td`)처럼 Semantics로 나타낼 수 없는 것에 쓴다.
class DomTagData {
  const DomTagData(this.tag, {this.fixed = false, this.attrs = const {}});
  final String tag;

  /// 쪽을 내려도 제자리(사이드바).
  final bool fixed;
  final Map<String, String> attrs;
}

class DomTag extends StatelessWidget {
  const DomTag(this.tag, {super.key, this.fixed = false, this.attrs = const {}, required this.child});
  final String tag;
  final bool fixed;
  final Map<String, String> attrs;
  final Widget child;

  @override
  Widget build(BuildContext context) =>
      MetaData(metaData: DomTagData(tag, fixed: fixed, attrs: attrs), behavior: HitTestBehavior.deferToChild, child: child);
}
