import 'package:anylive_mobile/navigation/app_routes.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('AppRoutes', () {
    test('room path builder', () {
      expect(AppRoutes.room('abc-123'), '/rooms/abc-123');
    });

    test('parseRoomId from path', () {
      expect(AppRoutes.parseRoomId('/rooms/r1'), 'r1');
      expect(AppRoutes.parseRoomId('/rooms/r1/'), 'r1');
      expect(AppRoutes.parseRoomId('/feed'), isNull);
      expect(AppRoutes.parseRoomId(null), isNull);
      expect(AppRoutes.parseRoomId(''), isNull);
    });
  });
}
