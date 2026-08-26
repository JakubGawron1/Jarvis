const String kLocalWs = 'ws://127.0.0.1:7420/ws';
const String kRenderWs = 'wss://jarvis-core-n12s.onrender.com/ws';
const String kPairingToken = String.fromEnvironment(
  'JARVIS_PAIRING_TOKEN',
  defaultValue: 'uMrUM1mJIQFOmGPwMVekLpsjBTwV9QcO1lsX/im7l5I=',
);

bool isRenderUplink(String url) => url.contains('onrender.com');

Map<String, dynamic> withToken(Map<String, dynamic> body) {
  if (kPairingToken.isEmpty) return body;
  return {...body, 'token': kPairingToken};
}
