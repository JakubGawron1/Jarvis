import 'dart:math' as math;
import 'package:flutter/material.dart';

class HudColors {
  static const bg = Color(0xFF050308);
  static const panel = Color(0x9E0E0602);
  static const panelOverlay = Color(0x6B0A0402);
  static const line = Color(0x73FF9A3C);
  static const amber = Color(0xFFFFB347);
  static const amberHot = Color(0xFFFF8C3A);
  static const gold = Color(0xFFFFE0B8);
  static const cyan = Color(0xFF4EE3FF);
  static const text = Color(0xFFFFE8CC);
  static const muted = Color(0xFFA07858);
}

TextStyle hudDisplay({
  double size = 11,
  Color color = HudColors.amber,
  double tracking = 3.2,
  FontWeight weight = FontWeight.w500,
}) {
  return TextStyle(
    fontSize: size,
    fontWeight: weight,
    letterSpacing: tracking,
    color: color,
    fontFamily: 'Orbitron',
    fontFamilyFallback: const ['Segoe UI', 'Roboto', 'sans-serif'],
  );
}

TextStyle hudMono({
  double size = 13,
  Color color = HudColors.text,
}) {
  return TextStyle(
    fontSize: size,
    letterSpacing: 0.4,
    color: color,
    height: 1.45,
    fontFamily: 'Consolas',
    fontFamilyFallback: const ['Courier New', 'monospace'],
  );
}

class HudGridPainter extends CustomPainter {
  const HudGridPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = const Color(0x0FFF8C3A)
      ..strokeWidth = 1;
    const step = 48.0;
    for (var x = 0.0; x < size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
    for (var y = 0.0; y < size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class HudScanlinePainter extends CustomPainter {
  const HudScanlinePainter();

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = const Color(0x2E000000);
    for (var y = 0.0; y < size.height; y += 3) {
      canvas.drawRect(Rect.fromLTWH(0, y + 2, size.width, 1), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class HudFramePainter extends CustomPainter {
  const HudFramePainter();

  @override
  void paint(Canvas canvas, Size size) {
    final hot = Paint()
      ..color = HudColors.amberHot
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.4
      ..strokeCap = StrokeCap.square;
    final inner = Paint()
      ..color = HudColors.amber.withValues(alpha: 0.55)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 0.8;
    const arm = 22.0;
    const inset = 10.0;
    void corner(double x, double y, double dx, double dy) {
      canvas.drawPath(
        Path()
          ..moveTo(x, y + arm * dy)
          ..lineTo(x, y)
          ..lineTo(x + arm * dx, y),
        hot,
      );
      canvas.drawPath(
        Path()
          ..moveTo(x + 4 * dx, y + (arm - 4) * dy)
          ..lineTo(x + 4 * dx, y + 4 * dy)
          ..lineTo(x + (arm - 4) * dx, y + 4 * dy),
        inner,
      );
    }

    corner(inset, inset, 1, 1);
    corner(size.width - inset, inset, -1, 1);
    corner(inset, size.height - inset, 1, -1);
    corner(size.width - inset, size.height - inset, -1, -1);

    final tick = Paint()
      ..color = HudColors.line
      ..strokeWidth = 1;
    final top = inset;
    final bot = size.height - inset;
    canvas.drawLine(Offset(size.width * 0.12, top), Offset(size.width * 0.88, top), tick);
    canvas.drawLine(Offset(size.width * 0.12, bot), Offset(size.width * 0.88, bot), tick);
    canvas.drawLine(Offset(inset, size.height * 0.22), Offset(inset, size.height * 0.78), tick);
    canvas.drawLine(
      Offset(size.width - inset, size.height * 0.22),
      Offset(size.width - inset, size.height * 0.78),
      tick,
    );
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class ArcReactorPainter extends CustomPainter {
  ArcReactorPainter({
    required this.t,
    required this.online,
    required this.cpu,
    required this.speaking,
  });

  final double t;
  final bool online;
  final double cpu;
  final bool speaking;

  @override
  void paint(Canvas canvas, Size size) {
    final c = Offset(size.width / 2, size.height / 2);
    final r = size.shortestSide / 2;
    canvas.save();
    canvas.translate(c.dx, c.dy);

    final spin = speaking ? t * 2.4 : t;
    final pulse = speaking
        ? 0.88 + 0.18 * math.sin(t * math.pi * 18)
        : 0.92 + 0.08 * math.sin(t * math.pi * 2);

    for (var i = 0; i < 220; i++) {
      final y = 1 - (i / 219) * 2;
      final rad = math.sqrt(math.max(0.0, 1 - y * y));
      final theta = i * 2.399963 + spin * math.pi * 2;
      final z = math.sin(theta) * rad;
      final x = math.cos(theta) * rad;
      final persp = 1 / (2.1 - z);
      final px = x * r * 0.78 * persp;
      final py = y * r * 0.78 * persp;
      final a = (0.15 + (z + 1) * 0.35).clamp(0.08, 0.85);
      canvas.drawCircle(
        Offset(px, py),
        (speaking ? 1.35 : 1.0) * persp * 1.4,
        Paint()..color = HudColors.amber.withValues(alpha: a),
      );
    }

    canvas.save();
    canvas.rotate(spin * math.pi * 2);
    canvas.drawCircle(
      Offset.zero,
      r * 0.72,
      Paint()
        ..style = PaintingStyle.stroke
        ..shader = SweepGradient(
          colors: const [HudColors.gold, HudColors.amberHot, HudColors.amber, HudColors.gold],
        ).createShader(Rect.fromCircle(center: Offset.zero, radius: r * 0.72))
        ..strokeWidth = speaking ? 8 : 5,
    );
    canvas.restore();

    canvas.drawCircle(
      Offset.zero,
      r * 0.22 * pulse,
      Paint()
        ..shader = RadialGradient(
          colors: [
            const Color(0xFFFFF3D6),
            HudColors.amber,
            const Color(0xFF3A1408),
          ],
        ).createShader(Rect.fromCircle(center: Offset.zero, radius: r * 0.28)),
    );

    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant ArcReactorPainter oldDelegate) =>
      oldDelegate.t != t ||
      oldDelegate.online != online ||
      oldDelegate.cpu != cpu ||
      oldDelegate.speaking != speaking;
}

class PanelClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) {
    const cut = 10.0;
    return Path()
      ..moveTo(cut, 0)
      ..lineTo(size.width, 0)
      ..lineTo(size.width, size.height - cut)
      ..lineTo(size.width - cut, size.height)
      ..lineTo(0, size.height)
      ..lineTo(0, cut)
      ..close();
  }

  @override
  bool shouldReclip(covariant CustomClipper<Path> oldClipper) => false;
}

class HudPanel extends StatelessWidget {
  const HudPanel({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return ClipPath(
      clipper: PanelClipper(),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: HudColors.panel,
          border: Border.all(color: HudColors.line),
        ),
        child: DecoratedBox(
          decoration: const BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [Color(0x1FFF9A3C), Color(0x00000000)],
              stops: [0, 0.32],
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

class ArcReactor extends StatefulWidget {
  const ArcReactor({
    super.key,
    required this.online,
    required this.cpu,
    this.speaking = false,
    this.size = 188,
  });

  final bool online;
  final double cpu;
  final bool speaking;
  final double size;

  @override
  State<ArcReactor> createState() => _ArcReactorState();
}

class _ArcReactorState extends State<ArcReactor> with SingleTickerProviderStateMixin {
  late final AnimationController _spin;

  @override
  void initState() {
    super.initState();
    _spin = AnimationController(vsync: this, duration: const Duration(seconds: 8))..repeat();
  }

  @override
  void didUpdateWidget(covariant ArcReactor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _spin.duration = Duration(milliseconds: widget.speaking ? 2200 : 8000);
    if (!_spin.isAnimating) _spin.repeat();
  }

  @override
  void dispose() {
    _spin.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: widget.size,
      height: widget.size,
      child: Stack(
        alignment: Alignment.center,
        children: [
          AnimatedBuilder(
            animation: _spin,
            builder: (context, _) => CustomPaint(
              size: Size.square(widget.size),
              painter: ArcReactorPainter(
                t: _spin.value,
                online: widget.online,
                cpu: widget.cpu,
                speaking: widget.speaking,
              ),
            ),
          ),
          Positioned(
            bottom: 14,
            child: Text(
              widget.speaking ? 'SPEAKING' : widget.online ? 'ONLINE' : 'STANDBY',
              style: hudDisplay(size: 9, color: HudColors.cyan, tracking: 3.4),
            ),
          ),
        ],
      ),
    );
  }
}
