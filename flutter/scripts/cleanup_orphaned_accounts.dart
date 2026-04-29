#!/usr/bin/env dart
// ignore_for_file: avoid_print
// Cleanup orphaned Rust account data
//
// This script finds and removes residual account data from the Rust vault
// that may prevent re-registration with the same account name.
//
// Usage:
//   dart run scripts/cleanup_orphaned_accounts.dart [--dry-run] [--list-only]
//
// Options:
//   --dry-run    Show what would be deleted without actually deleting
//   --list-only  Only list accounts without deleting anything

import 'dart:io';
import 'dart:convert';

void main(List<String> args) async {
  final dryRun = args.contains('--dry-run');
  final listOnly = args.contains('--list-only');

  print('=== SoloSoul Orphaned Accounts Cleanup ===\n');

  // Find the application support directory
  // On macOS, this is typically ~/Library/Application Support/
  final homeDir = Platform.environment['HOME'] ?? '';
  final appSupportPath = '$homeDir/Library/Application Support';

  // Try to find SoloSoul Flutter app data
  // The bundle ID is com.solosoul.solosoulFlutter
  // Also check common variations used during development
  final possiblePaths = [
    '$appSupportPath/com.solosoul.solosoulFlutter',
    '$appSupportPath/SoloSoulFlutter',
    '$appSupportPath/SoloSoul',
    '$appSupportPath/solosoul',
    '$homeDir/.solosoul',
    '$homeDir/solosoul_data',
  ];

  // Also check for data stored relative to executable (development mode)
  final exePath = Platform.resolvedExecutable;
  final exeDir = File(exePath).parent.path;
  final developmentPaths = [
    '$exeDir/../Frameworks',  // macOS app bundle
    '$exeDir/../../..',        // Deep development path
  ];

  print('Searching for SoloSoul data directories...\n');
  print('Home: $homeDir');
  print('App Support: $appSupportPath');
  print('Executable: $exePath\n');

  String? foundPath;
  for (final path in possiblePaths) {
    final dir = Directory(path);
    if (await dir.exists()) {
      foundPath = path;
      print('Found: $path');
      break;
    }
  }

  // Check development paths
  if (foundPath == null) {
    for (final basePath in developmentPaths) {
      for (final subPath in ['com.solosoul.solosoulFlutter', 'solosoul', '.solosoul']) {
        final fullPath = '$basePath/$subPath';
        final dir = Directory(fullPath);
        if (await dir.exists()) {
          foundPath = fullPath;
          print('Found (dev): $fullPath');
          break;
        }
      }
      if (foundPath != null) break;
    }
  }

  if (foundPath == null) {
    print('No SoloSoul application data directory found.');
    print('Searched in:');
    for (final path in possiblePaths) {
      print('  - $path');
    }
    print('\nNo cleanup needed.');
    return;
  }

  print('Found application data at: $foundPath\n');

  final baseDir = Directory(foundPath);
  final accountsFile = File('$foundPath/accounts.json');

  // Check if accounts.json exists
  if (!await accountsFile.exists()) {
    print('No accounts.json found - no Rust accounts to clean.');
  } else {
    // Read and parse accounts.json
    final content = await accountsFile.readAsString();
    List<dynamic>? rustAccounts;

    try {
      rustAccounts = jsonDecode(content) as List<dynamic>;
    } on Exception catch (e) {
      print('Error parsing accounts.json: $e');
      rustAccounts = null;
    }

    if (rustAccounts != null && rustAccounts.isNotEmpty) {
      print('Rust accounts in accounts.json:');
      for (final acc in rustAccounts) {
        final accMap = acc as Map<String, dynamic>;
        print('  - ID: ${accMap['id']}, Name: ${accMap['name']}');
      }
    }

    // List actual account directories
    print('\nAccount directories on disk:');
    final entities = await baseDir.list().toList();
    final accountDirs = entities.whereType<Directory>().where((d) {
      final name = d.path.split('/').last;
      return name.startsWith('acc_');
    }).toList();

    if (accountDirs.isEmpty) {
      print('  (none found)');
    } else {
      for (final dir in accountDirs) {
        final name = dir.path.split('/').last;
        print('  - $name');
      }
    }

    // Find orphaned directories (in accounts.json but not in Rust's view)
    if (rustAccounts != null) {
      final rustAccountIds = rustAccounts
          .map((a) => (a as Map<String, dynamic>)['id'] as String)
          .toSet();

      final orphanedDirs = accountDirs.where((d) {
        final name = d.path.split('/').last;
        return !rustAccountIds.contains(name);
      }).toList();

      if (orphanedDirs.isNotEmpty) {
        print('\n⚠️  Orphaned account directories (exist on disk but not in accounts.json):');
        for (final dir in orphanedDirs) {
          final name = dir.path.split('/').last;
          print('  - $name');
        }

        if (!listOnly && orphanedDirs.isNotEmpty) {
          print('\n');
          if (dryRun) {
            print('[DRY RUN] Would delete orphaned directories:');
            for (final dir in orphanedDirs) {
              print('  - ${dir.path}');
            }
          } else {
            print('Deleting orphaned directories...');
            for (final dir in orphanedDirs) {
              try {
                await dir.delete(recursive: true);
                print('  ✓ Deleted: ${dir.path}');
              } on Exception catch (e) {
                print('  ✗ Failed to delete ${dir.path}: $e');
              }
            }
          }
        }
      }
    }
  }

  // Also check for standalone account directories in home
  final homeSolosoul = Directory('$homeDir/.solosoul');
  if (await homeSolosoul.exists()) {
    print('\n⚠️  Found ~/.solosoul directory:');
    print('  Path: ${homeSolosoul.path}');

    final entities = await homeSolosoul.list().toList();
    final accountDirs = entities.whereType<Directory>().where((d) {
      final name = d.path.split('/').last;
      return name.startsWith('acc_');
    }).toList();

    if (accountDirs.isNotEmpty) {
      print('  Account directories:');
      for (final dir in accountDirs) {
        print('    - ${dir.path.split('/').last}');
      }
    }
  }

  print('\n=== Cleanup complete ===');
}
