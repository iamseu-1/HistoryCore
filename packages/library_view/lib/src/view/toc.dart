import 'package:flutter/widgets.dart';

import '../ui/dom.dart';
import '../ui/press.dart';
import '../ui/tokens.dart';
import '../ui/type.dart';
import 'model.dart';

/// 목차 — 왼쪽 가는 선(설계 3-4), 문단 번호와 제목. 누르면 그 문단으로.
class TocView extends StatelessWidget {
  const TocView({super.key, required this.sections, this.onSection});
  final List<Section> sections;
  final void Function(int section)? onSection;

  @override
  Widget build(BuildContext context) {
    if (sections.isEmpty) return const SizedBox.shrink();
    final top = sections.map((s) => s.level).reduce((a, b) => a < b ? a : b);
    return DomTag(
      'nav',
      attrs: const {'aria-label': '목차'},
      child: DecoratedBox(
        decoration: const BoxDecoration(border: Border(left: BorderSide(color: Ink.border))),
        child: Padding(
          padding: const EdgeInsets.only(left: 14),
          child: IntrinsicWidth(
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, mainAxisSize: MainAxisSize.min, children: [
              Padding(padding: const EdgeInsets.only(bottom: 2), child: Text('목차', style: TextStyles.of(12, 20, color: Ink.muted))),
              for (final s in sections)
                Press(
                  link: true,
                  label: '${s.number} ${s.title}',
                  onTap: onSection == null ? null : () => onSection!(s.index),
                  radius: Radii.chip,
                  padding: EdgeInsets.only(left: 16.0 * (s.level - top), right: 6),
                  child: Row(mainAxisSize: MainAxisSize.min, children: [
                    ConstrainedBox(constraints: const BoxConstraints(minWidth: 30), child: Text(s.number, style: TextStyles.tocNo)),
                    Flexible(child: Text(s.title, style: TextStyles.toc)),
                  ]),
                ),
            ]),
          ),
        ),
      ),
    );
  }
}
