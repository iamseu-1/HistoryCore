import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'tokens.dart';

/// 누를 수 있는 것 하나 — 실선 테두리 없이 칠로만 반응한다(설계 3-1: 실선 단추를 쓰지 않는다).
/// 올리면 [hover], 누르는 동안 [pressed], 키보드 포커스에는 2px 셋째 글자색 띠. 단추 · 링크로 읽힌다.
class Press extends StatefulWidget {
  const Press({
    super.key,
    required this.onTap,
    required this.child,
    this.label,
    this.link = false,
    this.fill,
    this.hover = Ink.hover,
    this.pressed = Ink.active,
    this.radius = Radii.control,
    this.padding = EdgeInsets.zero,
  });

  final VoidCallback? onTap;
  final Widget child;

  /// 읽어 줄 이름. 주면 자식의 글 대신 이것만 읽힌다.
  final String? label;
  final bool link;

  /// 평소 칠(주 동작의 옅은 회색 · 지금 위치).
  final Color? fill;
  final Color hover;
  final Color pressed;
  final double radius;
  final EdgeInsetsGeometry padding;

  @override
  State<Press> createState() => _PressState();
}

class _PressState extends State<Press> {
  var _hovered = false;
  var _down = false;
  var _ring = false;

  static const _keys = <ShortcutActivator, Intent>{
    SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
    SingleActivator(LogicalKeyboardKey.numpadEnter): ActivateIntent(),
    SingleActivator(LogicalKeyboardKey.space): ActivateIntent(),
  };

  final _focus = FocusNode(debugLabel: 'Press');

  @override
  void initState() {
    super.initState();
    _focus.addListener(_update);
    FocusManager.instance.addHighlightModeListener(_modeChanged);
  }

  @override
  void dispose() {
    FocusManager.instance.removeHighlightModeListener(_modeChanged);
    _focus.dispose();
    super.dispose();
  }

  void _modeChanged(FocusHighlightMode _) => _update();

  void _update() {
    // 키보드로 옮겨 왔을 때만 띠를 보인다(마우스로 누른 뒤에는 보이지 않음).
    final ring = _focus.hasPrimaryFocus && FocusManager.instance.highlightMode == FocusHighlightMode.traditional;
    if (ring != _ring && mounted) setState(() => _ring = ring);
  }

  @override
  Widget build(BuildContext context) {
    final enabled = widget.onTap != null;
    final color = !enabled ? widget.fill : (_down ? widget.pressed : (_hovered ? widget.hover : widget.fill));
    return Semantics(
      container: true,
      button: !widget.link,
      link: widget.link,
      enabled: enabled,
      label: widget.label,
      onTap: widget.onTap,
      child: Shortcuts(
        shortcuts: _keys,
        child: Actions(
          actions: {ActivateIntent: CallbackAction<ActivateIntent>(onInvoke: (_) => widget.onTap?.call())},
          child: Focus(
            focusNode: _focus,
            canRequestFocus: enabled,
            child: MouseRegion(
              cursor: enabled ? SystemMouseCursors.click : MouseCursor.defer,
              onEnter: (_) => setState(() => _hovered = true),
              onExit: (_) => setState(() => _hovered = false),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                excludeFromSemantics: true,
                onTap: widget.onTap,
                onTapDown: enabled ? (_) => setState(() => _down = true) : null,
                onTapUp: enabled ? (_) => setState(() => _down = false) : null,
                onTapCancel: enabled ? () => setState(() => _down = false) : null,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: color,
                    borderRadius: BorderRadius.circular(widget.radius),
                    border: _ring ? Border.all(color: Ink.muted, width: 2) : null,
                  ),
                  child: Padding(
                    padding: widget.padding,
                    child: widget.label == null ? widget.child : ExcludeSemantics(child: widget.child),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
