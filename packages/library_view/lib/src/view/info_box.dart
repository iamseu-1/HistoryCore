import 'package:flutter/widgets.dart';

import '../ui/dom.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';
import 'context.dart';
import 'inlines.dart';
import 'model.dart';

class InfoRow {
  const InfoRow(this.key, this.value);
  final String key;
  final List<Inline> value;
}

/// 정보 상자(R5) — 대상이 `틀:…정보`인 틀 블록 속 첫 표. 첫 줄은 제목(줄바꿈 뒤는 원어), 나머지 줄은 열쇠 | 값.
class InfoBoxData {
  const InfoBoxData({required this.blockIndex, required this.title, this.subtitle, required this.rows});

  /// 문서 블록 가운데 이 틀 블록의 자리 — 본문 흐름에서 뺀다.
  final int blockIndex;
  final String title;
  final String? subtitle;
  final List<InfoRow> rows;
}

InfoBoxData? infoBoxOf(DocModel m) {
  for (final (i, b) in m.blocks.indexed) {
    if (b is! TemplateBlock || b.status != 'ok' || !b.target.key.trim().endsWith('정보')) continue;
    final table = b.blocks.whereType<TableBlock>().firstOrNull;
    if (table == null || table.rows.isEmpty) continue;
    var rows = table.rows;
    var title = '';
    String? subtitle;
    if (rows.first.length == 1) {
      final inl = rows.first.single.inl;
      final br = inl.indexWhere((x) => x is BreakInline);
      title = plainTextOf(br < 0 ? inl : inl.sublist(0, br)).trim();
      if (br >= 0) subtitle = plainTextOf(inl.sublist(br + 1)).trim();
      if (subtitle?.isEmpty ?? false) subtitle = null;
      rows = rows.sublist(1);
    }
    return InfoBoxData(
      blockIndex: i,
      title: title,
      subtitle: subtitle,
      rows: [for (final r in rows) if (r.length >= 2) InfoRow(plainTextOf(r[0].inl).trim(), r[1].inl)],
    );
  }
  return null;
}

/// 제목 옆 원어(R4) — 첫 문단이 「제목(」 + 한자 구간으로 시작하면 그 한자.
String? originalTitle(DocModel m, String title) {
  final para = m.blocks.whereType<ParaBlock>().firstOrNull;
  if (para == null) return null;
  final inl = para.inl.skipWhile((i) => i is BreakInline).toList();
  if (inl.length < 2) return null;
  final head = inl[0], han = inl[1];
  if (head is TextInline && head.s.trimLeft() == '$title(' && han is HanInline) return han.s;
  return null;
}

/// 정보 상자 — 한 단계 바탕(무채색), 가운데 제목과 원어, 열쇠 56 | 값.
class InfoBoxView extends StatelessWidget {
  const InfoBoxView(this.data, {super.key, required this.ctx});
  final InfoBoxData data;
  final DocContext ctx;

  @override
  Widget build(BuildContext context) {
    return DomTag(
      'aside',
      attrs: const {'aria-label': '정보 상자'},
      child: DecoratedBox(
        decoration: BoxDecoration(color: Ink.raised, borderRadius: BorderRadius.circular(Radii.paper)),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(18, 16, 18, 6),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, mainAxisSize: MainAxisSize.min, children: [
            Text(data.title, style: TextStyles.infoTitle, textAlign: TextAlign.center),
            if (data.subtitle != null)
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(data.subtitle!, style: TextStyles.hanja(14, line: 20, color: Ink.muted), textAlign: TextAlign.center),
              ),
            const SizedBox(height: 12),
            for (final r in data.rows)
              DecoratedBox(
                decoration: const BoxDecoration(border: Border(top: BorderSide(color: Ink.border))),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 9),
                  child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    SizedBox(width: 56, child: Text(r.key, style: TextStyles.infoKey)),
                    const SizedBox(width: 12),
                    Expanded(child: ParaView(r.value, ctx: ctx, style: TextStyles.infoValue)),
                  ]),
                ),
              ),
          ]),
        ),
      ),
    );
  }
}
