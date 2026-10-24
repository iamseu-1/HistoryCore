// 끌어 선택하고 복사(P3-6 R5) — Flutter의 선택은 선택된 글 조각을 그냥 이어 붙인다. 블록마다 줄바꿈, 표는 줄마다 칸을 탭으로 이어
// 평문이 읽는 차례대로 나오게 한다. 각주 표시 · 단추 · 목차 · 번호는 선택에서 뺀다(SelectionContainer.disabled).
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// 블록 하나 — 선택된 글 끝에 줄바꿈(이미 줄바꿈으로 끝나면 더하지 않는다 — 표가 넣은 것).
class BlockSelectionDelegate extends StaticSelectionContainerDelegate {
  @override
  SelectedContent? getSelectedContent() {
    final c = super.getSelectedContent();
    // 선택이 스쳤을 뿐 글이 없는 블록은 빈 줄을 만들지 않는다.
    if (c == null || c.plainText.isEmpty) return null;
    return c.plainText.endsWith('\n') ? c : SelectedContent(plainText: '${c.plainText}\n');
  }
}

// 칸이 표에 제 자리를 알리는 표지 — 글에 나올 수 없는 제어 글자. 표 대리자가 벗겨 내므로 클립보드에 가지 않는다.
const _cellMark = '\u0001';
const _cellSep = '\u0002';
const _cellEnd = '\u0003';

/// 표 칸 하나 — 선택된 글 앞에 (줄, 칸) 표지를 붙인다.
class CellSelectionDelegate extends StaticSelectionContainerDelegate {
  CellSelectionDelegate({required this.row, required this.col});
  int row;
  int col;

  @override
  SelectedContent? getSelectedContent() {
    final c = super.getSelectedContent();
    if (c == null) return null;
    return SelectedContent(plainText: '$_cellMark$row$_cellSep$col$_cellEnd${c.plainText}');
  }
}

/// 표 하나 — 칸들의 표지를 읽어, 선택된 줄마다 첫 선택 칸부터 끝 선택 칸까지 빈 칸도 자리를 지켜 탭으로 잇고 줄바꿈(검토 I3).
/// 캡션처럼 칸이 아닌 글은 제 줄.
class TableSelectionDelegate extends StaticSelectionContainerDelegate {
  @override
  SelectedContent? getSelectedContent() {
    final rows = <int, Map<int, String>>{};
    final others = <String>[];
    for (final s in selectables) {
      final c = s.getSelectedContent();
      if (c == null) continue;
      final text = c.plainText;
      if (text.startsWith(_cellMark)) {
        final sep = text.indexOf(_cellSep), end = text.indexOf(_cellEnd);
        final r = int.tryParse(text.substring(1, sep)), k = int.tryParse(text.substring(sep + 1, end));
        if (r != null && k != null) rows.putIfAbsent(r, () => {})[k] = text.substring(end + 1);
      } else if (text.isNotEmpty) {
        others.add(text.endsWith('\n') ? text : '$text\n');
      }
    }
    if (rows.isEmpty && others.isEmpty) return null;
    final b = StringBuffer()..writeAll(others);
    for (final r in rows.keys.toList()..sort()) {
      final cols = rows[r]!;
      // 선택이 스쳤을 뿐 글이 없는 줄(머리 줄의 끝 같은 것)은 빈 줄을 만들지 않는다.
      if (cols.values.every((t) => t.isEmpty)) continue;
      final from = cols.keys.reduce((a, c) => a < c ? a : c), to = cols.keys.reduce((a, c) => a > c ? a : c);
      b.write([for (var k = from; k <= to; k++) cols[k] ?? ''].join('\t'));
      b.write('\n');
    }
    return SelectedContent(plainText: b.toString());
  }
}

/// 블록을 선택 단위로 감싼다 — 대리자는 한 번 만들어 둔다.
class SelectionBlock extends StatefulWidget {
  const SelectionBlock({super.key, required this.child});
  final Widget child;

  @override
  State<SelectionBlock> createState() => _SelectionBlockState();
}

class _SelectionBlockState extends State<SelectionBlock> {
  final _delegate = BlockSelectionDelegate();

  @override
  void dispose() {
    _delegate.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SelectionContainer(delegate: _delegate, child: widget.child);
}

/// 표를 선택 단위로 감싼다 — 안의 [SelectionCell]들이 표지로 제 자리를 알린다.
class SelectionTable extends StatefulWidget {
  const SelectionTable({super.key, required this.child});
  final Widget child;

  @override
  State<SelectionTable> createState() => _SelectionTableState();
}

class _SelectionTableState extends State<SelectionTable> {
  final _delegate = TableSelectionDelegate();

  @override
  void dispose() {
    _delegate.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SelectionContainer(delegate: _delegate, child: widget.child);
}

/// 표 칸 하나 — (줄, 칸). 자리가 바뀌면 대리자를 새로 만들지 않고 값만 바꾼다(아래 글 조각들이 옛 대리자에서 빠져나오다 단언에 걸린다).
class SelectionCell extends StatefulWidget {
  const SelectionCell({super.key, required this.row, required this.col, required this.child});
  final int row;
  final int col;
  final Widget child;

  @override
  State<SelectionCell> createState() => _SelectionCellState();
}

class _SelectionCellState extends State<SelectionCell> {
  late final _delegate = CellSelectionDelegate(row: widget.row, col: widget.col);

  @override
  void didUpdateWidget(SelectionCell old) {
    super.didUpdateWidget(old);
    _delegate
      ..row = widget.row
      ..col = widget.col;
  }

  @override
  void dispose() {
    _delegate.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SelectionContainer(delegate: _delegate, child: widget.child);
}
