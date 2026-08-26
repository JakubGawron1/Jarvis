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
  ArcReactorPainter({required this.t, required this.online, required this.cpu});

  final double t;
  final bool online;
  final double cpu;

  @override
  void paint(Canvas canvas, Size size) {
    final c = Offset(size.width / 2, size.height / 2);
    final r = size.shortestSide / 2;
    canvas.save();
    canvas.translate(c.dx, c.dy);

    final gold = Paint()
      ..style = PaintingStyle.stroke
      ..color = HudColors.amber
      ..strokeWidth = 1.2;
    canvas.drawCircle(Offset.zero, r * 0.96, gold..color = HudColors.amber.withValues(alpha: 0.22));

    for (var i = 0; i < 36; i++) {
      final a = (i / 36) * math.pi * 2;
      final inner = i % 6 == 0 ? r * 0.88 : r * 0.91;
      canvas.drawLine(
        Offset(math.cos(a) * inner, math.sin(a) * inner),
        Offset(math.cos(a) * r * 0.96, math.sin(a) * r * 0.96),
        Paint()
          ..color = i % 6 == 0 ? HudColors.amber : HudColors.amber.withValues(alpha: 0.4)
          ..strokeWidth = i % 6 == 0 ? 1.4 : 0.7,
      );
    }

    canvas.save();
    canvas.rotate(-t * math.pi * 2 / 22);
    canvas.drawCircle(
      Offset.zero,
      r * 0.82,
      Paint()
        ..style = PaintingStyle.stroke
        ..color = HudColors.amberHot
        ..strokeWidth = 1.4
        ..strokeCap = StrokeCap.round,
    );
    canvas.restore();

    canvas.drawCircle(
      Offset.zero,
      r * 0.72,
      Paint()
        ..style = PaintingStyle.stroke
        ..shader = SweepGradient(
          colors: const [HudColors.gold, HudColors.amberHot, HudColors.amber, HudColors.gold],
        ).createShader(Rect.fromCircle(center: Offset.zero, radius: r * 0.72))
        ..strokeWidth = 7,
    );

    canvas.save();
    canvas.rotate(t * math.pi * 2);
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: r * 0.58),
      0,
      math.pi * 1.4,
      false,
      Paint()
        ..style = PaintingStyle.stroke
        ..color = const Color(0xFFFF6A1A)
        ..strokeWidth = 2
        ..strokeCap = StrokeCap.square,
    );
    canvas.restore();

    canvas.save();
    canvas.rotate(t * math.pi * 2 / 18);
    final hex = Path();
    for (var i = 0; i < 6; i++) {
      final a = (math.pi / 3) * i - math.pi / 2;
      final p = Offset(math.cos(a) * r * 0.34, math.sin(a) * r * 0.34);
      if (i == 0) {
        hex.moveTo(p.dx, p.dy);
      } else {
        hex.lineTo(p.dx, p.dy);
      }
    }
    hex.close();
    canvas.drawPath(
      hex,
      Paint()
        ..style = PaintingStyle.stroke
        ..color = HudColors.cyan.withValues(alpha: 0.55)
        ..strokeWidth = 1.2,
    );
    canvas.restore();

    final sweep = (cpu.clamp(8, 100) / 100) * math.pi * 2;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: r * 0.46),
      -math.pi / 2,
      sweep,
      false,
      Paint()
        ..style = PaintingStyle.stroke
        ..color = HudColors.cyan.withValues(alpha: 0.45)
        ..strokeWidth = 9
        ..strokeCap = StrokeCap.round,
    );

    canvas.save();
    canvas.rotate(t * math.pi * 2 * 8 / 4.8);
    canvas.drawPath(
      Path()
        ..moveTo(0, 0)
        ..lineTo(0, -r * 0.82)
        ..arcToPoint(
          Offset(r * 0.42, -r * 0.7),
          radius: Radius.circular(r * 0.82),
        )
        ..close(),
      Paint()..color = HudColors.cyan.withValues(alpha: online ? 0.14 : 0.06),
    );
    canvas.restore();

    final pulse = 0.85 + 0.15 * math.sin(t * math.pi * 2 * (online ? 1.4 : 0.7));
    canvas.drawCircle(
      Offset.zero,
      r * 0.28 * pulse,
      Paint()
        ..shader = RadialGradient(
          colors: [
            const Color(0xFFE8FBFF),
            HudColors.cyan,
            const Color(0xFF0A3A4A),
          ],
        ).createShader(Rect.fromCircle(center: Offset.zero, radius: r * 0.28)),
    );
    canvas.drawCircle(Offset.zero, r * 0.14, Paint()..color = const Color(0xFF041018));
    canvas.drawCircle(Offset.zero, r * 0.06, Paint()..color = HudColors.cyan);

    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant ArcReactorPainter oldDelegate) =>
      oldDelegate.t != t || oldDelegate.online != online || oldDelegate.cpu != cpu;
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
  const ArcReactor({super.key, required this.online, required this.cpu, this.size = 188});

  final bool online;
  final double cpu;
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
              ),
            ),
          ),
          Positioned(
            bottom: 14,
            child: Text(
              widget.online ? 'ONLINE' : 'STANDBY',
              style: hudDisplay(size: 9, color: HudColors.cyan, tracking: 3.4),
            ),
          ),
        ],
      ),
    );
  }
}
