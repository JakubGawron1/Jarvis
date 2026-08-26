import 'package:flutter_test/flutter_test.dart';
import 'package:jarvis_app/main.dart';

void main() {
  testWidgets('HUD loads', (tester) async {
    await tester.pumpWidget(const JarvisApp());
    await tester.pump();
    expect(find.textContaining('J.A.R.V.I.S.'), findsOneWidget);
    expect(find.textContaining('JARVIS'), findsOneWidget);
  });
}
