import 'dart:convert';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'hud_chrome.dart';

class VisualTake {
  const VisualTake(this.text, this.spec);
  final String text;
  final Map<String, dynamic>? spec;
}

/// Pull `[[visual:{...}]]` out of a reply so chat never dumps raw JSON.
VisualTake takeVisual(String raw) {
  const marker = '[[visual:';
  final start = raw.indexOf(marker);
  if (start < 0) return VisualTake(raw.trim(), null);

  final jsonStart = raw.indexOf('{', start);
  if (jsonStart < 0) {
    return VisualTake((raw.substring(0, start) + raw.substring(start + marker.length)).trim(), null);
  }

  var depth = 0;
  var inStr = false;
  var escape = false;
  var jsonEnd = -1;
  for (var i = jsonStart; i < raw.length; i++) {
    final c = raw[i];
    if (inStr) {
      if (escape) {
        escape = false;
        continue;
      }
      if (c == r'\') {
        escape = true;
        continue;
      }
      if (c == '"') inStr = false;
      continue;
    }
    if (c == '"') {
      inStr = true;
      continue;
    }
    if (c == '{') depth++;
    if (c == '}') {
      depth--;
      if (depth == 0) {
        jsonEnd = i;
        break;
      }
    }
  }

  Map<String, dynamic>? spec;
  if (jsonEnd >= 0) {
    try {
      final decoded = jsonDecode(raw.substring(jsonStart, jsonEnd + 1));
      if (decoded is Map<String, dynamic>) spec = decoded;
    } catch (_) {}
  }

  var end = jsonEnd >= 0 ? jsonEnd + 1 : raw.length;
  final rest = raw.substring(end).trimLeft();
  if (rest.startsWith(']]')) {
    end += raw.substring(end).length - rest.length + 2;
  }
  final text = (raw.substring(0, start) + raw.substring(end)).trim();
  return VisualTake(text, spec);
}

class VisualStage extends StatefulWidget {
  const VisualStage({super.key, required this.spec, required this.onDismiss});

  final Map<String, dynamic> spec;
  final VoidCallback onDismiss;

  @override
  State<VisualStage> createState() => _VisualStageState();
}

class _VisualStageState extends State<VisualStage> with SingleTickerProviderStateMixin {
  late final AnimationController _tick;
  var _slide = 0;

  @override
  void initState() {
    super.initState();
    _tick = AnimationController(vsync: this, duration: const Duration(seconds: 24))..repeat();
  }

  @override
  void didUpdateWidget(covariant VisualStage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.spec['title'] != widget.spec['title'] || oldWidget.spec['kind'] != widget.spec['kind']) {
      _slide = 0;
    }
  }

  @override
  void dispose() {
    _tick.dispose();
    super.dispose();
  }

  String get _kind => '${widget.spec['kind'] ?? 'scene3d'}';
  String get _title => '${widget.spec['title'] ?? 'Hologram'}';
  String get _subtitle => '${widget.spec['subtitle'] ?? ''}';

  List<Map<String, dynamic>> get _facts {
    final raw = widget.spec['facts'];
    if (raw is! List) return const [];
    return raw.whereType<Map>().map((e) => Map<String, dynamic>.from(e)).toList();
  }

  List<Map<String, dynamic>> get _slides {
    final raw = widget.spec['slides'];
    if (raw is! List) return const [];
    return raw.whereType<Map>().map((e) => Map<String, dynamic>.from(e)).toList();
  }

  Map<String, dynamic>? get _diagram {
    final raw = widget.spec['diagram'];
    if (raw is Map) return Map<String, dynamic>.from(raw);
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final slides = _slides;
    return ColoredBox(
      color: const Color(0xE6050308),
      child: Stack(
        children: [
          const Positioned.fill(child: CustomPaint(painter: HudGridPainter())),
          Positioned.fill(
            child: AnimatedBuilder(
              animation: _tick,
              builder: (_, _) => CustomPaint(
                painter: _HoloPainter(spec: widget.spec, t: _tick.value * math.pi * 2),
              ),
            ),
          ),
          Positioned(
            left: 18,
            right: 18,
            top: 18,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _kind.replaceAll('3d', ' 3D').toUpperCase(),
                  style: hudDisplay(size: 10, color: HudColors.cyan, tracking: 3.2),
                ),
                const SizedBox(height: 6),
                Text(_title, style: hudDisplay(size: 16, color: HudColors.gold, tracking: 1.6, weight: FontWeight.w600)),
                if (_subtitle.isNotEmpty)
                  Text(_subtitle, style: hudMono(size: 12, color: HudColors.muted)),
                if (_facts.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: [
                      for (final f in _facts)
                        DecoratedBox(
                          decoration: BoxDecoration(
                            border: Border.all(color: HudColors.line),
                            color: HudColors.panelOverlay,
                          ),
                          child: Padding(
                            padding: const EdgeInsets.fromLTRB(12, 10, 16, 10),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text('${f['label'] ?? ''}', style: hudDisplay(size: 9, color: HudColors.cyan, tracking: 2.4)),
                                const SizedBox(height: 4),
                                Text('${f['value'] ?? ''}', style: hudDisplay(size: 22, color: HudColors.gold, tracking: 1.2, weight: FontWeight.w600)),
                              ],
                            ),
                          ),
                        ),
                    ],
                  ),
                ],
              ],
            ),
          ),
          if (_kind == 'slides' && slides.isNotEmpty)
            Align(
              alignment: Alignment.bottomCenter,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(24, 0, 24, 88),
                child: _SlideCard(
                  slide: slides[_slide.clamp(0, slides.length - 1)],
                  index: _slide,
                  total: slides.length,
                  onPrev: () => setState(() => _slide = math.max(0, _slide - 1)),
                  onNext: () => setState(() => _slide = math.min(slides.length - 1, _slide + 1)),
                ),
              ),
            ),
          if (_kind == 'diagram' && _diagram != null)
            Align(
              alignment: Alignment.bottomCenter,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(24, 0, 24, 88),
                child: _DiagramCard(diagram: _diagram!),
              ),
            ),
          Positioned(
            right: 18,
            bottom: 18,
            child: OutlinedButton(
              onPressed: widget.onDismiss,
              style: OutlinedButton.styleFrom(
                foregroundColor: HudColors.amber,
                side: const BorderSide(color: HudColors.line),
                shape: const RoundedRectangleBorder(borderRadius: BorderRadius.zero),
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                textStyle: hudDisplay(size: 11, tracking: 2.2),
              ),
              child: const Text('ZAMKNIJ HOLOGRAM'),
            ),
          ),
        ],
      ),
    );
  }
}

class _SlideCard extends StatelessWidget {
  const _SlideCard({
    required this.slide,
    required this.index,
    required this.total,
    required this.onPrev,
    required this.onNext,
  });

  final Map<String, dynamic> slide;
  final int index;
  final int total;
  final VoidCallback onPrev;
  final VoidCallback onNext;

  @override
  Widget build(BuildContext context) {
    final bullets = (slide['bullets'] as List?)?.map((e) => '$e').toList() ?? const <String>[];
    return HudPanel(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${slide['title'] ?? ''}', style: hudDisplay(size: 14, color: HudColors.gold)),
            const SizedBox(height: 10),
            for (final b in bullets)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text('▸ $b', style: hudMono(size: 13, color: HudColors.text)),
              ),
            Row(
              children: [
                TextButton(onPressed: onPrev, child: const Text('←')),
                Text('${index + 1} / $total', style: hudMono(size: 12, color: HudColors.muted)),
                TextButton(onPressed: onNext, child: const Text('→')),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _DiagramCard extends StatelessWidget {
  const _DiagramCard({required this.diagram});
  final Map<String, dynamic> diagram;

  @override
  Widget build(BuildContext context) {
    final nodes = (diagram['nodes'] as List?)?.map((e) => '$e').toList() ?? const <String>[];
    return HudPanel(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final n in nodes)
              DecoratedBox(
                decoration: BoxDecoration(
                  border: Border.all(color: HudColors.line),
                  color: HudColors.panel,
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  child: Text(n, style: hudMono(size: 12, color: HudColors.gold)),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _Vec3 {
  const _Vec3(this.x, this.y, this.z);
  final double x, y, z;
}

class _HoloPainter extends CustomPainter {
  _HoloPainter({required this.spec, required this.t});
  final Map<String, dynamic> spec;
  final double t;

  @override
  void paint(Canvas canvas, Size size) {
    final scene = spec['scene3d'];
    final bodies = <Map<String, dynamic>>[];
    var links = <List<int>>[];
    var particles = 160;
    var neural = false;
    var camZ = 8.0;
    if (scene is Map) {
      camZ = (scene['camera_z'] as num?)?.toDouble() ?? 8.0;
      particles = (scene['particles'] as num?)?.toInt() ?? 160;
      neural = scene['neural'] == true;
      final rawBodies = scene['bodies'];
      if (rawBodies is List) {
        for (final b in rawBodies) {
          if (b is Map) bodies.add(Map<String, dynamic>.from(b));
        }
      }
      final rawLinks = scene['links'];
      if (rawLinks is List) {
        for (final l in rawLinks) {
          if (l is List && l.length >= 2) {
            links.add([(l[0] as num).toInt(), (l[1] as num).toInt()]);
          }
        }
      }
    }

    final positions = <_Vec3>[];
    for (var i = 0; i < bodies.length; i++) {
      positions.add(_bodyPos(bodies[i], i, t));
    }

    _drawParticles(canvas, size, t, particles.clamp(40, 900), camZ);

    if (neural || bodies.isEmpty) {
      _drawRing(canvas, size, t, camZ);
    }

    for (final pair in links) {
      if (pair[0] < 0 || pair[1] < 0 || pair[0] >= positions.length || pair[1] >= positions.length) {
        continue;
      }
      final a = _project(positions[pair[0]], size, t, camZ);
      final b = _project(positions[pair[1]], size, t, camZ);
      canvas.drawLine(
        a.$1,
        b.$1,
        Paint()
          ..color = const Color(0x47FF8C3A)
          ..strokeWidth = 1.2,
      );
    }

    final order = List<int>.generate(positions.length, (i) => i);
    order.sort((a, b) => positions[b].z.compareTo(positions[a].z));
    for (final i in order) {
      _drawBody(canvas, size, bodies[i], positions[i], t, camZ);
    }
  }

  _Vec3 _bodyPos(Map<String, dynamic> b, int i, double t) {
    final pos = b['position'];
    var ox = 0.0, oy = 0.0, oz = 0.0;
    if (pos is List && pos.length >= 3) {
      ox = (pos[0] as num).toDouble();
      oy = (pos[1] as num).toDouble();
      oz = (pos[2] as num).toDouble();
    }
    final orbit = b['orbit'];
    if (orbit is! Map) return _Vec3(ox, oy, oz);
    final r = (orbit['radius'] as num?)?.toDouble() ?? 1.5;
    final speed = (orbit['speed'] as num?)?.toDouble() ?? 1.0;
    final tilt = (orbit['tilt'] as num?)?.toDouble() ?? 0.0;
    final ang = t * speed * (0.7 + (i % 5) * 0.05) + i * 0.4;
    if (tilt.abs() > 1.6) {
      return _Vec3(ox + math.cos(ang) * r, oy + tilt, oz + math.sin(ang) * r);
    }
    return _Vec3(
      ox + math.cos(ang) * r,
      oy + math.sin(ang * 0.35) * r * 0.12,
      oz + math.sin(ang) * r,
    );
  }

  (Offset, double) _project(_Vec3 p, Size size, double t, double camZ) {
    final cy = math.cos(t * 0.08);
    final sy = math.sin(t * 0.08);
    final x1 = p.x * cy - p.z * sy;
    final z1 = p.x * sy + p.z * cy;
    final y1 = p.y;
    final f = (size.shortestSide * 0.42) / (camZ + z1).clamp(1.2, 40);
    return (
      Offset(size.width / 2 + x1 * f, size.height * 0.48 + y1 * f),
      f,
    );
  }

  Color _hex(String raw) {
    var h = raw.replaceFirst('#', '');
    if (h.length == 6) h = 'FF$h';
    if (h.length != 8) return HudColors.amberHot;
    return Color(int.parse(h, radix: 16));
  }

  void _drawBody(Canvas canvas, Size size, Map<String, dynamic> b, _Vec3 p, double t, double camZ) {
    final (o, f) = _project(p, size, t, camZ);
    final radius = ((b['radius'] as num?)?.toDouble() ?? 0.25) * f * 0.85;
    final color = _hex('${b['color'] ?? '#ff8c3a'}');
    final glow = b['glow'] == true;
    if (glow) {
      canvas.drawCircle(
        o,
        radius * 2.4,
        Paint()
          ..color = color.withValues(alpha: 0.18)
          ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 12),
      );
    }
    final g = ui.Gradient.radial(
      o.translate(-radius * 0.25, -radius * 0.28),
      radius * 1.2,
      [Color.lerp(Colors.white, color, 0.25)!, color, Color.lerp(color, Colors.black, 0.45)!],
      const [0.0, 0.45, 1.0],
    );
    canvas.drawCircle(o, radius.clamp(2.0, 64.0), Paint()..shader = g);
    canvas.drawCircle(
      o,
      radius.clamp(2.0, 64.0),
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1
        ..color = color.withValues(alpha: 0.7),
    );
    final label = b['label'];
    if (label is String && label.isNotEmpty) {
      final tp = TextPainter(
        text: TextSpan(text: label, style: hudMono(size: 10, color: HudColors.gold)),
        textDirection: TextDirection.ltr,
      )..layout();
      tp.paint(canvas, o.translate(-tp.width / 2, radius + 4));
    }
  }

  void _drawParticles(Canvas canvas, Size size, double t, int n, double camZ) {
    final paint = Paint()..color = const Color(0xB3FFB347);
    final rng = math.Random(7);
    for (var i = 0; i < n; i++) {
      final r = 1.2 + rng.nextDouble() * 6.5;
      final th = rng.nextDouble() * math.pi * 2 + t * 0.05;
      final ph = math.acos(2 * rng.nextDouble() - 1);
      final p = _Vec3(
        r * math.sin(ph) * math.cos(th),
        r * math.cos(ph) * 0.7,
        r * math.sin(ph) * math.sin(th),
      );
      final (o, f) = _project(p, size, t, camZ);
      canvas.drawCircle(o, (0.9 + f * 0.015).clamp(0.6, 2.2), paint);
    }
  }

  void _drawRing(Canvas canvas, Size size, double t, double camZ) {
    final pts = <Offset>[];
    for (var i = 0; i <= 72; i++) {
      final a = i / 72 * math.pi * 2;
      final p = _Vec3(math.cos(a) * 3.2, math.sin(a) * 0.4, math.sin(a) * 3.2);
      pts.add(_project(p, size, t, camZ).$1);
    }
    canvas.drawPoints(
      ui.PointMode.polygon,
      pts,
      Paint()
        ..color = const Color(0x59FF8C3A)
        ..strokeWidth = 1.2
        ..style = PaintingStyle.stroke,
    );
  }

  @override
  bool shouldRepaint(covariant _HoloPainter oldDelegate) => oldDelegate.t != t || oldDelegate.spec != spec;
}
