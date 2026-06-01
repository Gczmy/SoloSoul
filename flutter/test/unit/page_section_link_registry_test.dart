import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/page_section_link_registry.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

void main() {
  group('PageSectionLinkRegistry', () {
    test('getDefaultSectionIdsForPage returns sections for profile', () {
      final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(
        DefaultPageIds.profile,
      );
      expect(sections, isNotEmpty);
      expect(sections, contains(DefaultSectionIds.identity));
      expect(sections, contains(DefaultSectionIds.contact));
    });

    test('getDefaultSectionIdsForPage returns sections for travel', () {
      final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(
        DefaultPageIds.travel,
      );
      expect(sections, contains(DefaultSectionIds.passport));
      expect(sections, contains(DefaultSectionIds.visa));
    });

    test('getDefaultSectionIdsForPage returns sections for financial', () {
      final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(
        DefaultPageIds.financial,
      );
      expect(sections, contains(DefaultSectionIds.bankAccount));
    });

    test('getDefaultSectionIdsForPage returns sections for professional', () {
      final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(
        DefaultPageIds.professional,
      );
      expect(sections, contains(DefaultSectionIds.education));
      expect(sections, contains(DefaultSectionIds.employment));
    });

    test('getDefaultSectionIdsForPage returns empty for unknown page', () {
      expect(
        PageSectionLinkRegistry.getDefaultSectionIdsForPage('unknown'),
        isEmpty,
      );
    });

    test('allDefaultLinks contains all pages', () {
      final links = PageSectionLinkRegistry.allDefaultLinks;
      expect(links.keys, contains(DefaultPageIds.profile));
      expect(links.keys, contains(DefaultPageIds.travel));
      expect(links.keys, contains(DefaultPageIds.financial));
      expect(links.keys, contains(DefaultPageIds.professional));
    });

    test('allDefaultLinks is unmodifiable', () {
      final links = PageSectionLinkRegistry.allDefaultLinks;
      expect(() => links['new'] = [], throwsUnsupportedError);
    });

    test('getDefaultPageIdForSection finds correct page', () {
      expect(
        PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.identity),
        DefaultPageIds.profile,
      );
      expect(
        PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.passport),
        DefaultPageIds.travel,
      );
      expect(
        PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.bankAccount),
        DefaultPageIds.financial,
      );
      expect(
        PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.education),
        DefaultPageIds.professional,
      );
    });

    test('getDefaultPageIdForSection returns null for unknown section', () {
      expect(
        PageSectionLinkRegistry.getDefaultPageIdForSection('unknown'),
        isNull,
      );
    });

    test('getDefaultStructureTree returns list of maps', () {
      final tree = PageSectionLinkRegistry.getDefaultStructureTree();
      expect(tree.length, 4);
      for (final entry in tree) {
        expect(entry.containsKey('pageId'), isTrue);
        expect(entry.containsKey('sectionIds'), isTrue);
        expect(entry['sectionIds'], isA<List<String>>());
      }
    });
  });
}
