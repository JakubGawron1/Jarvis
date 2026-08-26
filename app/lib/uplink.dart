import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

const String kLocalWs = 'ws://127.0.0.1:7420/ws';
const String kRenderWs = 'wss://jarvis-core-n12s.onrender.com/ws';
const String kLanWs = String.fromEnvironment('JARVIS_LAN_WS');
const String kPairingToken = String.fromEnvironment(
  'JARVIS_PAIRING_TOKEN',
  defaultValue: 'uMrUM1mJIQFOmGPwMVekLpsjBTwV9QcO1lsX/im7l5I=',
);

bool isRenderUplink(String url) => url.contains('onrender.com');

Map<String, dynamic> withToken(Map<String, dynamic> body) {
  if (kPairingToken.isEmpty) return body;
  return {...body, 'token': kPairingToken};
}

bool get _isPhone =>
    defaultTargetPlatform == TargetPlatform.android || defaultTargetPlatform == TargetPlatform.iOS;

/// Phone never uses 127.0.0.1 (that is the handset). Desktop tries local first.
List<String> uplinkCandidates() {
  final lan = kLanWs.trim();
  if (_isPhone) {
    return [
      if (lan.isNotEmpty) lan,
      kRenderWs,
    ];
  }
  return [
    kLocalWs,
    if (lan.isNotEmpty) lan,
    kRenderWs,
  ];
}

String healthUrl(String ws) {
  var u = ws.replaceFirst(RegExp(r'^ws'), 'http');
  if (u.endsWith('/ws')) {
    u = '${u.substring(0, u.length - 3)}/health';
  } else if (!u.endsWith('/health')) {
    u = '$u/health';
  }
  return u;
}

String uplinkLabel(String url) {
  if (isRenderUplink(url)) return 'Render';
  if (url.contains('127.0.0.1')) return 'Local';
  return 'LAN';
}

Future<void> kickHealth(String wsUrl) async {
  final href = healthUrl(wsUrl);
  try {
    final client = HttpClient()..connectionTimeout = const Duration(seconds: 15);
    final req = await client.getUrl(Uri.parse(href));
    req.headers.set(HttpHeaders.userAgentHeader, 'jarvis-flutter');
    final res = await req.close().timeout(const Duration(seconds: 40));
    await res.drain<void>();
    client.close(force: true);
  } catch (_) {}
}

Future<WebSocketChannel?> connectUplink(
  String url, {
  required void Function(String) onLog,
}) async {
  final render = isRenderUplink(url);
  final attempts = render ? 10 : 1;
  final each = Duration(seconds: render ? 20 : 3);

  if (render) {
    onLog('Budzę Render… to może zająć minutę.');
    await kickHealth(url);
  }

  for (var i = 1; i <= attempts; i++) {
    onLog('Łączę ${uplinkLabel(url)} ($i/$attempts)…');
    if (render && i > 1) await kickHealth(url);
    WebSocket? socket;
    try {
      socket = await WebSocket.connect(
        url,
        headers: {'Origin': 'https://jarvis-core-n12s.onrender.com'},
      ).timeout(each);
      socket.pingInterval = const Duration(seconds: 20);
      onLog('Uplink: ${uplinkLabel(url)}');
      return IOWebSocketChannel(socket);
    } catch (e) {
      try {
        await socket?.close();
      } catch (_) {}
      onLog('Brak ${uplinkLabel(url)}: ${_shortErr(e)}');
      if (i < attempts) {
        await Future<void>.delayed(const Duration(seconds: 2));
      }
    }
  }
  return null;
}

String _shortErr(Object e) {
  final s = e.toString().replaceAll('\n', ' ');
  if (s.length <= 160) return s;
  return s.substring(0, 160);
}
