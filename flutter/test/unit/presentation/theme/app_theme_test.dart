import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

void main() {
  group('SnackBarType', () {
    test('has expected values', () {
      expect(SnackBarType.values, hasLength(4));
      expect(SnackBarType.values, contains(SnackBarType.info));
      expect(SnackBarType.values, contains(SnackBarType.success));
      expect(SnackBarType.values, contains(SnackBarType.warning));
      expect(SnackBarType.values, contains(SnackBarType.error));
    });

    test('values have correct index order', () {
      expect(SnackBarType.info.index, 0);
      expect(SnackBarType.success.index, 1);
      expect(SnackBarType.warning.index, 2);
      expect(SnackBarType.error.index, 3);
    });
  });
}
