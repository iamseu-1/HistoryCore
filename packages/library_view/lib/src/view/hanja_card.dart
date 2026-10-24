import 'package:flutter/widgets.dart';

import '../ui/press.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';

/// 자전 카드 한 장의 자료(설계 3-4 · 목업 ha-view의 카드).
class HanjaCard {
  const HanjaCard({
    required this.char,
    required this.readings,
    required this.radical,
    required this.radicalStrokes,
    required this.strokes,
    required this.source,
    required this.gloss,
    required this.status,
  });
  final String char;

  /// 음훈 — 「학 학」.
  final String readings;
  final String radical;

  /// 부수 밖 획수.
  final int radicalStrokes;
  final int strokes;

  /// 《康熙字典》 원문 첫머리.
  final String source;

  /// 번역 첫머리.
  final String gloss;

  /// 번역 상태 — 「모델 번역 · 검토 전」.
  final String status;
}

/// 자전을 여는 딥링크 — `historyarchive://kangxi/<글자>`(HistoryCore spec/windows-api.md 6-1). 글자는 퍼센트로 싼다.
/// 「자전에서 열기」를 누르면 받는 앱(HistoryTime)이 이 링크를 연다 — 패키지는 링크를 열지 않는다(P5 R8).
Uri kangxiUri(String char) => Uri(scheme: 'historyarchive', host: 'kangxi', pathSegments: [char]);

/// 자전 카드 — 떠 있는 것이라 그림자가 있다(설계 3-1). [card]가 없으면 글자와 「자전에서 열기」만
/// ([missing]을 주면 글자 곁에 그 한 줄 — 아카이브의 「표본 자전에 없는 글자」).
class HanjaCardView extends StatelessWidget {
  const HanjaCardView({super.key, required this.char, this.card, required this.onOpen, this.width = 320, this.docked = false, this.missing});
  final String char;
  final HanjaCard? card;
  final VoidCallback onOpen;
  final double? width;

  /// 자료가 없을 때 글자 곁에 보일 한 줄. null이면 글자 칸과 「자전에서 열기」만.
  final String? missing;

  /// 폰의 아래 시트 — 아래 모서리를 둥글리지 않는다.
  final bool docked;

  @override
  Widget build(BuildContext context) {
    final c = card;
    return SizedBox(
      width: width,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Ink.surface,
          borderRadius: docked ? const BorderRadius.vertical(top: Radius.circular(Radii.paper)) : BorderRadius.circular(Radii.paper),
          boxShadow: Ink.floatShadow,
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, mainAxisSize: MainAxisSize.min, children: [
            Row(children: [
              DecoratedBox(
                decoration: BoxDecoration(color: Ink.raised, borderRadius: BorderRadius.circular(Radii.control)),
                child: SizedBox(width: 64, height: 64, child: Center(child: Text(char, style: TextStyles.hanja(44, line: 52)))),
              ),
              if (c != null || missing != null) ...[
                const SizedBox(width: 14),
                Expanded(
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    Text(c?.readings ?? char, style: TextStyles.cardTitle),
                    const SizedBox(height: 2),
                    Text(
                      c == null ? missing! : '${c.radical}부 ${c.radicalStrokes}획 · 총 ${c.strokes}획',
                      style: TextStyles.of(12.5, 18, color: Ink.muted),
                    ),
                  ]),
                ),
              ],
            ]),
            if (c != null) ...[
              const SizedBox(height: 12),
              Text(c.source, style: TextStyles.hanja(14.5, line: 24), maxLines: 1, overflow: TextOverflow.ellipsis),
              Text(c.gloss, style: TextStyles.of(13, 20, color: Ink.ink2), maxLines: 3, overflow: TextOverflow.ellipsis),
            ],
            const SizedBox(height: 10),
            const SizedBox(height: 1, child: ColoredBox(color: Ink.rule)),
            const SizedBox(height: 4),
            Row(children: [
              Press(
                link: true,
                onTap: onOpen,
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Text('자전에서 열기', style: TextStyles.of(12.5, 18, weight: FontWeight.w600)),
              ),
              const Spacer(),
              if (c != null) Text(c.status, style: TextStyles.of(12.5, 18, color: Ink.muted)),
            ]),
          ]),
        ),
      ),
    );
  }
}
