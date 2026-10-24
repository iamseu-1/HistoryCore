import 'dart:math' as math;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// 오른쪽에 띄운 상자(정보 상자) 곁으로 흐르는 블록들 — CSS float를 블록 단위로.
/// 상자 곁에서 시작한 블록은 좁은 폭(전체 − 상자 − 틈), 상자 아래에서 시작한 블록은 온 폭.
class FloatLayout extends MultiChildRenderObjectWidget {
  FloatLayout({super.key, required Widget float, required List<Widget> flow, this.floatWidth = 300, this.gap = 32, this.floatTop = 28, this.floatBottom = 20})
      : super(children: [float, ...flow]);
  final double floatWidth;
  final double gap;
  final double floatTop;
  final double floatBottom;

  @override
  RenderObject createRenderObject(BuildContext context) => RenderFloatLayout(floatWidth, gap, floatTop, floatBottom);

  @override
  void updateRenderObject(BuildContext context, RenderFloatLayout renderObject) {
    renderObject
      ..floatWidth = floatWidth
      ..gap = gap
      ..floatTop = floatTop
      ..floatBottom = floatBottom;
  }
}

class FloatParentData extends ContainerBoxParentData<RenderBox> {}

class RenderFloatLayout extends RenderBox
    with ContainerRenderObjectMixin<RenderBox, FloatParentData>, RenderBoxContainerDefaultsMixin<RenderBox, FloatParentData> {
  RenderFloatLayout(this._floatWidth, this._gap, this._floatTop, this._floatBottom);

  double _floatWidth, _gap, _floatTop, _floatBottom;
  set floatWidth(double v) => _set(() => _floatWidth = v, _floatWidth != v);
  set gap(double v) => _set(() => _gap = v, _gap != v);
  set floatTop(double v) => _set(() => _floatTop = v, _floatTop != v);
  set floatBottom(double v) => _set(() => _floatBottom = v, _floatBottom != v);
  void _set(VoidCallback f, bool changed) {
    if (!changed) return;
    f();
    markNeedsLayout();
  }

  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! FloatParentData) child.parentData = FloatParentData();
  }

  @override
  void performLayout() {
    final w = constraints.maxWidth;
    final float = firstChild;
    if (float == null) {
      size = constraints.constrain(Size(w, 0));
      return;
    }
    float.layout(BoxConstraints.tightFor(width: _floatWidth), parentUsesSize: true);
    (float.parentData! as FloatParentData).offset = Offset(w - _floatWidth, _floatTop);
    final floatEnd = _floatTop + float.size.height + _floatBottom;
    var y = 0.0;
    for (var c = childAfter(float); c != null; c = childAfter(c)) {
      final narrow = y < floatEnd;
      c.layout(BoxConstraints(maxWidth: narrow ? math.max(0, w - _floatWidth - _gap) : w), parentUsesSize: true);
      (c.parentData! as FloatParentData).offset = Offset(0, y);
      y += c.size.height;
    }
    size = constraints.constrain(Size(w, math.max(y, floatEnd)));
  }

  @override
  double computeMinIntrinsicWidth(double height) => _floatWidth;
  @override
  double computeMaxIntrinsicWidth(double height) => _floatWidth * 3;

  @override
  void paint(PaintingContext context, Offset offset) => defaultPaint(context, offset);

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) => defaultHitTestChildren(result, position: position);
}
