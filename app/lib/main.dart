import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/material.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'hud_chrome.dart';

void main() => runApp(const JarvisApp());

class JarvisApp extends StatelessWidget {
  const JarvisApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'J.A.R.V.I.S.',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        useMaterial3: true,
        scaffoldBackgroundColor: HudColors.bg,
        colorScheme: const ColorScheme.dark(
          primary: HudColors.cyan,
          secondary: HudColors.amber,
          surface: HudColors.bg,
        ),
        inputDecorationTheme: InputDecorationTheme(
          filled: true,
          fillColor: const Color(0xBF080301),
          hintStyle: hudMono(size: 12, color: HudColors.muted),
          border: const OutlineInputBorder(
            borderRadius: BorderRadius.zero,
            borderSide: BorderSide(color: HudColors.line),
          ),
          enabledBorder: const OutlineInputBorder(
            borderRadius: BorderRadius.zero,
            borderSide: BorderSide(color: HudColors.line),
          ),
          focusedBorder: const OutlineInputBorder(
            borderRadius: BorderRadius.zero,
            borderSide: BorderSide(color: HudColors.cyan),
          ),
          contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        ),
      ),
      home: const HudPage(),
    );
  }
}

class _Msg {
  const _Msg(this.role, this.text);
  final String role;
  final String text;
}

class HudPage extends StatefulWidget {
  const HudPage({super.key});

  @override
  State<HudPage> createState() => _HudPageState();
}

class _HudPageState extends State<HudPage> {
  final _ctrl = TextEditingController();
  final _urlCtrl = TextEditingController(text: 'ws://127.0.0.1:7420/ws');
  final _scroll = ScrollController();
  final _log = <_Msg>[
    const _Msg('sys', 'Interface online. Address me in Polish or English.'),
  ];
  WebSocketChannel? _ch;
  String _status = 'offline';
  String _leader = '—';
  String _io = '—';
  String _clock = '--:--:--';
  double _cpu = 0;
  String _ram = '—';
  String _model = '—';
  String _version = '—';
  Timer? _clockTimer;
  final _devices = <Map<String, dynamic>>[];
  var _alive = true;
  final _player = AudioPlayer();

  String _pad(int n) => n.toString().padLeft(2, '0');

  void _tickClock() {
    if (!mounted) return;
    final d = DateTime.now();
    setState(() {
      _clock = '${_pad(d.hour)}:${_pad(d.minute)}:${_pad(d.second)}';
    });
  }

  void _connect() {
    _ch?.sink.close();
    try {
      _ch = WebSocketChannel.connect(Uri.parse(_urlCtrl.text));
      if (mounted) setState(() => _status = 'online');
      _ch!.stream.listen((event) {
        if (!_alive) return;
        final m = jsonDecode(event as String) as Map<String, dynamic>;
        final t = m['type'];
        setState(() {
          if (t == 'reply') {
            _log.add(_Msg('jarvis', '${m['content']}'));
          } else if (t == 'confirm') {
            _log.add(_Msg('sys', 'CONFIRM: ${m['prompt']}'));
          } else if (t == 'job_deferred') {
            _log.add(_Msg('sys', '${m['message']}'));
          } else if (t == 'presence') {
            _leader = '${m['leader'] ?? '—'}';
            _io = '${m['io_device'] ?? '—'}';
            _devices
              ..clear()
              ..addAll(((m['devices'] as List?) ?? []).cast<Map<String, dynamic>>());
          } else if (t == 'error') {
            _log.add(_Msg('sys', 'error: ${m['message']}'));
          } else if (t == 'stats') {
            _cpu = (m['cpu'] as num?)?.toDouble() ?? 0;
            final used = ((m['ram_used'] as num?) ?? 0) / 1e6;
            final total = ((m['ram_total'] as num?) ?? 0) / 1e6;
            _ram = '${used.round()} / ${total.round()} MB';
            _model = '${m['model'] ?? '—'}';
            _version = '${m['core_version'] ?? '—'}';
          }
        });
        if (t == 'speech') {
          final b64 = m['audio_b64'] as String?;
          if (b64 != null) {
            _playSpeech(b64, '${m['mime'] ?? 'audio/mpeg'}');
          }
        }
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted || !_scroll.hasClients) return;
          _scroll.animateTo(
            _scroll.position.maxScrollExtent,
            duration: const Duration(milliseconds: 280),
            curve: Curves.easeOut,
          );
        });
      }, onDone: () {
        if (!_alive) return;
        setState(() => _status = 'offline');
      }, onError: (_) {
        if (!_alive) return;
        setState(() => _status = 'offline');
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _status = 'offline';
        _log.add(_Msg('sys', '$e'));
      });
    }
  }

  Future<void> _playSpeech(String b64, String mime) async {
    try {
      final bytes = Uint8List.fromList(base64Decode(b64));
      await _player.stop();
      await _player.play(BytesSource(bytes, mimeType: mime));
    } catch (_) {}
  }

  void _send(String text) {
    if (text.trim().isEmpty) return;
    setState(() => _log.add(_Msg('user', text)));
    _ch?.sink.add(jsonEncode({
      'type': 'text',
      'id': Random().nextInt(1 << 30).toString(),
      'content': text,
    }));
    _ctrl.clear();
  }

  @override
  void initState() {
    super.initState();
    _tickClock();
    _clockTimer = Timer.periodic(const Duration(seconds: 1), (_) => _tickClock());
    _connect();
  }

  @override
  void dispose() {
    _alive = false;
    _clockTimer?.cancel();
    _player.dispose();
    final ch = _ch;
    _ch = null;
    ch?.sink.close();
    _ctrl.dispose();
    _urlCtrl.dispose();
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final online = _status == 'online';
    return Scaffold(
      body: Stack(
        children: [
          const DecoratedBox(
            decoration: BoxDecoration(
              gradient: RadialGradient(
                center: Alignment(0, -0.55),
                radius: 1.05,
                colors: [Color(0x291C0A06), HudColors.bg],
              ),
            ),
            child: SizedBox.expand(),
          ),
          const Positioned.fill(child: CustomPaint(painter: _Grid())),
          const Positioned.fill(child: IgnorePointer(child: CustomPaint(painter: _Scan()))),
          const Positioned.fill(child: CustomPaint(painter: _Frame())),
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(18, 10, 18, 12),
              child: Column(
                children: [
                  _TopBar(clock: _clock, status: _status),
                  const SizedBox(height: 10),
                  Expanded(
                    child: LayoutBuilder(
                      builder: (context, box) {
                        final wide = box.maxWidth >= 720;
                        final reactor = _ReactorColumn(
                          online: online,
                          cpu: _cpu,
                          ram: _ram,
                          model: _model,
                          version: _version,
                          io: _io,
                          leader: _leader,
                          devices: _devices,
                          urlCtrl: _urlCtrl,
                          onLink: _connect,
                          compact: !wide,
                        );
                        final chat = _ChatColumn(
                          log: _log,
                          scroll: _scroll,
                          ctrl: _ctrl,
                          onSend: _send,
                        );
                        if (!wide) {
                          return Column(
                            children: [
                              SizedBox(height: 168, child: reactor),
                              const SizedBox(height: 10),
                              Expanded(child: chat),
                            ],
                          );
                        }
                        return Row(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            SizedBox(width: 292, child: reactor),
                            const SizedBox(width: 14),
                            Expanded(child: chat),
                          ],
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Grid extends HudGridPainter {
  const _Grid();
}

class _Scan extends HudScanlinePainter {
  const _Scan();
}

class _Frame extends HudFramePainter {
  const _Frame();
}

class _TopBar extends StatelessWidget {
  const _TopBar({required this.clock, required this.status});
  final String clock;
  final String status;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Text(
          'J.A.R.V.I.S.',
          style: hudDisplay(size: 15, color: HudColors.gold, tracking: 4.2, weight: FontWeight.w600),
        ),
        const Spacer(),
        Text(clock, style: hudDisplay(size: 12, color: HudColors.cyan, tracking: 2.4)),
        const SizedBox(width: 18),
        Text(
          status == 'online' ? 'JARVIS  ·  STARK OS' : 'JARVIS  ·  STANDBY',
          style: hudDisplay(size: 10, color: HudColors.amber.withValues(alpha: 0.75), tracking: 3.4),
        ),
      ],
    );
  }
}

class _ReactorColumn extends StatelessWidget {
  const _ReactorColumn({
    required this.online,
    required this.cpu,
    required this.ram,
    required this.model,
    required this.version,
    required this.io,
    required this.leader,
    required this.devices,
    required this.urlCtrl,
    required this.onLink,
    required this.compact,
  });

  final bool online;
  final double cpu;
  final String ram;
  final String model;
  final String version;
  final String io;
  final String leader;
  final List<Map<String, dynamic>> devices;
  final TextEditingController urlCtrl;
  final VoidCallback onLink;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    return HudPanel(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: compact
            ? Row(
                children: [
                  ArcReactor(online: online, cpu: cpu, size: 120),
                  const SizedBox(width: 12),
                  Expanded(child: SingleChildScrollView(child: _stats())),
                ],
              )
            : ListView(
                children: [
                  Text('ARC REACTOR', style: hudDisplay()),
                  Center(child: ArcReactor(online: online, cpu: cpu)),
                  _stats(),
                  const SizedBox(height: 10),
                  Text('UPLINK', style: hudDisplay()),
                  const SizedBox(height: 8),
                  TextField(controller: urlCtrl, style: hudMono(size: 12)),
                  const SizedBox(height: 8),
                  _HudButton(label: 'Link', onPressed: onLink),
                ],
              ),
      ),
    );
  }

  Widget _stats() {
    Widget row(String k, String v) => Padding(
          padding: const EdgeInsets.only(bottom: 7),
          child: DecoratedBox(
            decoration: const BoxDecoration(
              border: Border(bottom: BorderSide(color: Color(0x2EFF9A3C), style: BorderStyle.solid)),
            ),
            child: Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                children: [
                  Text(k, style: hudMono(size: 12, color: HudColors.muted)),
                  const Spacer(),
                  Flexible(
                    child: Text(
                      v,
                      overflow: TextOverflow.ellipsis,
                      style: hudMono(size: 12, color: HudColors.gold),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        row('CPU', '${cpu.toStringAsFixed(0)}%'),
        row('RAM', ram),
        row('Model', model),
        row('Core', version),
        row('I/O', io),
        row('Leader', leader),
        if (devices.isEmpty)
          Text('No active nodes', style: hudMono(size: 12, color: HudColors.amberHot))
        else
          ...devices.map(
            (d) => Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                '${d['name']} · ${d['kind']}',
                style: hudMono(
                  size: 12,
                  color: d['id'] == leader ? HudColors.cyan : HudColors.text,
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _ChatColumn extends StatelessWidget {
  const _ChatColumn({
    required this.log,
    required this.scroll,
    required this.ctrl,
    required this.onSend,
  });

  final List<_Msg> log;
  final ScrollController scroll;
  final TextEditingController ctrl;
  final void Function(String) onSend;

  Color _roleColor(String role) {
    switch (role) {
      case 'user':
        return HudColors.gold;
      case 'jarvis':
        return HudColors.cyan;
      default:
        return HudColors.muted;
    }
  }

  @override
  Widget build(BuildContext context) {
    return HudPanel(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text('CONVERSATION', style: hudDisplay()),
            Expanded(
              child: ListView.builder(
                controller: scroll,
                itemCount: log.length,
                itemBuilder: (_, i) {
                  final m = log[i];
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 7),
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          left: BorderSide(color: _roleColor(m.role), width: 2),
                        ),
                      ),
                      child: Padding(
                        padding: const EdgeInsets.only(left: 10),
                        child: RichText(
                          text: TextSpan(
                            children: [
                              TextSpan(
                                text: '${m.role.toUpperCase()}  ',
                                style: hudDisplay(size: 9, color: _roleColor(m.role), tracking: 2.2),
                              ),
                              TextSpan(text: m.text, style: hudMono(color: _roleColor(m.role))),
                            ],
                          ),
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
            const SizedBox(height: 10),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: ctrl,
                    style: hudMono(),
                    decoration: const InputDecoration(hintText: 'Awaiting input — PL / EN'),
                    onSubmitted: onSend,
                  ),
                ),
                const SizedBox(width: 8),
                _HudButton(label: 'Send', onPressed: () => onSend(ctrl.text)),
                const SizedBox(width: 8),
                _MicButton(onPressed: () => onSend(ctrl.text.isEmpty ? 'Jarvis, status' : ctrl.text)),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _HudButton extends StatelessWidget {
  const _HudButton({required this.label, required this.onPressed});
  final String label;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton(
      onPressed: onPressed,
      style: OutlinedButton.styleFrom(
        foregroundColor: HudColors.amber,
        side: const BorderSide(color: HudColors.line),
        shape: const RoundedRectangleBorder(borderRadius: BorderRadius.zero),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 16),
        textStyle: hudDisplay(size: 11, tracking: 2.2),
      ),
      child: Text(label.toUpperCase()),
    );
  }
}

class _MicButton extends StatelessWidget {
  const _MicButton({required this.onPressed});
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 52,
      height: 52,
      child: OutlinedButton(
        onPressed: onPressed,
        style: OutlinedButton.styleFrom(
          foregroundColor: HudColors.cyan,
          side: const BorderSide(color: Color(0x804EE3FF)),
          shape: const CircleBorder(),
          padding: EdgeInsets.zero,
        ),
        child: Text('MIC', style: hudDisplay(size: 9, color: HudColors.cyan, tracking: 1.6)),
      ),
    );
  }
}
