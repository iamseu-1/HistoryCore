import 'package:flutter/widgets.dart';

import '../ui/format.dart';
import '../ui/icons.dart';
import '../ui/press.dart';
import '../ui/svg.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';

/// 문서 머리(목업 ha-view) — 제목과 원어, 고친 날, 넘겨주기, 분류 줄(첫 분류에 폴더), 관련 문서 줄.
class DocHeadView extends StatelessWidget {
  const DocHeadView({
    super.key,
    required this.title,
    this.original,
    this.updated,
    this.redirectedFrom,
    this.categories = const [],
    this.related = const [],
    this.phone = false,
    this.onConflict,
  });
  final String title;
  final String? original;

  /// 고친 날 — null이면 그 줄을 뺀다(받는 앱이 모를 때).
  final DateTime? updated;
  final String? redirectedFrom;

  /// 동기화 겹침(P4-2)이면 — 띠의 「견주어 합치기」(편집의 견주기로).
  final VoidCallback? onConflict;

  /// (이름, 분류 문서가 있는가, 누르면) — 첫 것이 집.
  final List<(String, bool, VoidCallback)> categories;
  final List<(String, VoidCallback)> related;
  final bool phone;

  @override
  Widget build(BuildContext context) {
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, mainAxisSize: MainAxisSize.min, children: [
      Wrap(
        spacing: phone ? 10 : 14,
        runSpacing: 4,
        crossAxisAlignment: WrapCrossAlignment.end,
        children: [
          Semantics(header: true, headingLevel: 1, child: Text(title, style: phone ? TextStyles.h1Phone : TextStyles.h1)),
          if (original != null)
            Padding(padding: const EdgeInsets.only(bottom: 3), child: Text(original!, style: TextStyles.hanja(19, line: 26, color: Ink.muted))),
        ],
      ),
      if (updated != null || redirectedFrom != null) const SizedBox(height: 6),
      if (updated case final u?) Text('${koreanDate(u)} 고침', style: TextStyles.meta),
      if (redirectedFrom != null) Text('$redirectedFrom에서 넘어옴', style: TextStyles.meta),
      if (onConflict case final open?) ...[
        const SizedBox(height: 12),
        DecoratedBox(
          key: const ValueKey('sync-conflict'),
          decoration: const BoxDecoration(color: Ink.raised, border: Border(left: BorderSide(color: Ink.border, width: 2))),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(14, 6, 6, 6),
            child: Row(children: [
              Expanded(child: Text('다른 기기에서 고친 판과 겹침 — 늦게 고친 판을 보이고 있습니다.', style: TextStyles.of(13, 20))),
              Press(link: true, onTap: open, padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6), child: Text('견주어 합치기', style: TextStyles.action)),
            ]),
          ),
        ),
      ],
      if (categories.isNotEmpty) ...[
        const SizedBox(height: 14),
        _line('분류', [
          for (final (i, (name, exists, onTap)) in categories.indexed)
            KeyedSubtree(key: ValueKey('category-$name'), child: _link(name, exists, onTap, folder: i == 0)),
        ]),
      ],
      if (related.isNotEmpty) ...[
        const SizedBox(height: 4),
        _line('관련 문서', [
          for (final (name, onTap) in related) KeyedSubtree(key: ValueKey('related-$name'), child: _link(name, true, onTap)),
        ]),
      ],
    ]);
  }

  Widget _line(String key, List<Widget> items) {
    final label = Text(key, style: TextStyles.of(13, 20, color: Ink.muted));
    final list = Wrap(crossAxisAlignment: WrapCrossAlignment.center, runSpacing: 4, children: [
      for (final (i, w) in items.indexed) ...[
        if (i > 0) Padding(padding: const EdgeInsets.symmetric(horizontal: 6), child: Text('·', style: TextStyles.of(14, 20, color: Ink.icon))),
        w,
      ],
    ]);
    if (phone) {
      return Wrap(crossAxisAlignment: WrapCrossAlignment.center, spacing: 8, runSpacing: 4, children: [label, list]);
    }
    return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [SizedBox(width: 74, child: label), Expanded(child: list)]);
  }

  static Widget _link(String name, bool exists, VoidCallback onTap, {bool folder = false}) {
    final style = exists
        ? TextStyles.line.copyWith(decoration: TextDecoration.underline, decorationColor: Ink.linkLine)
        : TextStyles.line.copyWith(color: Ink.muted, decoration: TextDecoration.underline, decorationStyle: TextDecorationStyle.dotted, decorationColor: Ink.missingLine);
    return Press(
      link: true,
      label: name,
      onTap: onTap,
      radius: Radii.chip,
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        if (folder) ...[const AppIcon(Glyphs.folder, size: 14), const SizedBox(width: 5)],
        Flexible(child: Text(name, style: style)),
      ]),
    );
  }
}

/// 머리줄의 분류 길 — 「동양사 › 한국사 › 조선 › 조선 선조」, 끝은 먹.
class CrumbView extends StatelessWidget {
  const CrumbView({super.key, required this.path, required this.onTap});
  final List<String> path;
  final void Function(String category) onTap;

  @override
  Widget build(BuildContext context) => Row(mainAxisSize: MainAxisSize.min, children: [
        for (final (i, name) in path.indexed) ...[
          if (i > 0) const Padding(padding: EdgeInsets.symmetric(horizontal: 6), child: AppIcon(Glyphs.chevronRight, size: 9)),
          Flexible(
            child: KeyedSubtree(
              key: ValueKey('crumb-$name'),
              child: Press(
                link: true,
                label: name,
                onTap: () => onTap(name),
                radius: Radii.chip,
                child: Text(name, style: i == path.length - 1 ? TextStyles.crumb.copyWith(color: Ink.text) : TextStyles.crumb, maxLines: 1, overflow: TextOverflow.ellipsis),
              ),
            ),
          ),
        ],
      ]);
}

/// 머리줄의 글자 단추(연표에서 보기 · 판 기록 n · 가리키는 문서 n).
class HeadAction extends StatelessWidget {
  const HeadAction({super.key, required this.label, this.count, this.cap, this.glyph, required this.onTap});
  final String label;
  final int? count;

  /// 질의의 한도 — 닿으면 「n+」.
  final int? cap;
  final SvgScene? glyph;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => Press(
        link: true,
        label: count == null ? label : '$label ${countLabel(count!, cap: cap)}',
        onTap: onTap,
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: SizedBox(
          height: Metrics.control,
          child: Row(mainAxisSize: MainAxisSize.min, children: [
            if (glyph != null) ...[AppIcon(glyph!, size: 15), const SizedBox(width: 6)],
            Text.rich(TextSpan(text: count == null ? label : '$label ', style: TextStyles.action, children: [
              if (count != null) TextSpan(text: countLabel(count!, cap: cap), style: TextStyles.action.copyWith(color: Ink.muted, fontFeatures: const [FontFeature.tabularFigures()])),
            ])),
          ]),
        ),
      );
}

/// 주 동작(편집) — 옅은 회색 칠, 실선 없음.
class PrimaryAction extends StatelessWidget {
  const PrimaryAction({super.key, required this.label, this.glyph, required this.onTap});
  final String label;
  final SvgScene? glyph;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => Press(
        label: label,
        onTap: onTap,
        fill: Ink.active,
        hover: Ink.control,
        pressed: Ink.control,
        padding: const EdgeInsets.symmetric(horizontal: 14),
        child: SizedBox(
          height: Metrics.control,
          child: Row(mainAxisSize: MainAxisSize.min, children: [
            if (glyph != null) ...[AppIcon(glyph!, size: 14, color: Ink.text), const SizedBox(width: 6)],
            Text(label, style: TextStyles.primary),
          ]),
        ),
      );
}
