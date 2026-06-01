import 'package:flutter_test/flutter_test.dart';
import 'package:solosoul_flutter/core/services/page_section_link_registry.dart';
import 'package:solosoul_flutter/core/services/unified_object_service.dart';

void main() {
  group('PageSectionLinkRegistry', () {
    group('getDefaultSectionIdsForPage', () {
      test('returns sections for profile page', () {
        final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(DefaultPageIds.profile);
        expect(sections, isNotEmpty);
        expect(sections, contains(DefaultSectionIds.identity));
        expect(sections, contains(DefaultSectionIds.contact));
        expect(sections, contains(DefaultSectionIds.idCard));
        expect(sections, contains(DefaultSectionIds.address));
      });

      test('returns sections for travel page', () {
        final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(DefaultPageIds.travel);
        expect(sections, contains(DefaultSectionIds.passport));
        expect(sections, contains(DefaultSectionIds.visa));
        expect(sections, contains(DefaultSectionIds.travelHistory));
      });

      test('returns sections for financial page', () {
        final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(DefaultPageIds.financial);
        expect(sections, contains(DefaultSectionIds.bankAccount));
        expect(sections, contains(DefaultSectionIds.card));
        expect(sections, contains(DefaultSectionIds.taxId));
      });

      test('returns sections for professional page', () {
        final sections = PageSectionLinkRegistry.getDefaultSectionIdsForPage(DefaultPageIds.professional);
        expect(sections, contains(DefaultSectionIds.education));
        expect(sections, contains(DefaultSectionIds.employment));
        expect(sections, contains(DefaultSectionIds.skill));
        expect(sections, contains(DefaultSectionIds.language));
        expect(sections, contains(DefaultSectionIds.award));
        expect(sections, contains(DefaultSectionIds.article));
      });

      test('returns empty list for unknown page', () {
        expect(PageSectionLinkRegistry.getDefaultSectionIdsForPage('unknown'), isEmpty);
      });
    });

    group('allDefaultLinks', () {
      test('returns all page links', () {
        final links = PageSectionLinkRegistry.allDefaultLinks;
        expect(links.keys, hasLength(4));
        expect(links.containsKey(DefaultPageIds.profile), isTrue);
        expect(links.containsKey(DefaultPageIds.travel), isTrue);
        expect(links.containsKey(DefaultPageIds.financial), isTrue);
        expect(links.containsKey(DefaultPageIds.professional), isTrue);
      });

      test('returns unmodifiable map', () {
        final links = PageSectionLinkRegistry.allDefaultLinks;
        expect(() => links['new'] = [], throwsUnsupportedError);
      });
    });

    group('getDefaultPageIdForSection', () {
      test('finds page for identity section', () {
        final pageId = PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.identity);
        expect(pageId, DefaultPageIds.profile);
      });

      test('finds page for passport section', () {
        final pageId = PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.passport);
        expect(pageId, DefaultPageIds.travel);
      });

      test('finds page for bank account section', () {
        final pageId = PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.bankAccount);
        expect(pageId, DefaultPageIds.financial);
      });

      test('finds page for education section', () {
        final pageId = PageSectionLinkRegistry.getDefaultPageIdForSection(DefaultSectionIds.education);
        expect(pageId, DefaultPageIds.professional);
      });

      test('returns null for unknown section', () {
        expect(PageSectionLinkRegistry.getDefaultPageIdForSection('unknown'), isNull);
      });
    });

    group('getDefaultStructureTree', () {
      test('returns tree with all pages', () {
        final tree = PageSectionLinkRegistry.getDefaultStructureTree();
        expect(tree, hasLength(4));
        final pageIds = tree.map((e) => e['pageId']).toSet();
        expect(pageIds, contains(DefaultPageIds.profile));
        expect(pageIds, contains(DefaultPageIds.travel));
        expect(pageIds, contains(DefaultPageIds.financial));
        expect(pageIds, contains(DefaultPageIds.professional));
      });

      test('each tree node has sectionIds list', () {
        final tree = PageSectionLinkRegistry.getDefaultStructureTree();
        for (final node in tree) {
          expect(node['pageId'], isA<String>());
          expect(node['sectionIds'], isA<List<String>>());
        }
      });
    });
  });
}
