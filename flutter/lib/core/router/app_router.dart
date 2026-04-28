import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:solosoul_flutter/presentation/pages/splash_page.dart';
import 'package:solosoul_flutter/presentation/pages/login_page.dart';
import 'package:solosoul_flutter/presentation/pages/home_page.dart';
import 'package:solosoul_flutter/presentation/pages/profile_page.dart';
import 'package:solosoul_flutter/presentation/pages/travel_page.dart';
import 'package:solosoul_flutter/presentation/pages/financial_page.dart';
import 'package:solosoul_flutter/presentation/pages/professional_page.dart';
import 'package:solosoul_flutter/presentation/pages/settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/data_management_page.dart';
import 'package:solosoul_flutter/presentation/pages/security_settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/operation_log_page.dart';
import 'package:solosoul_flutter/presentation/pages/sensitivity_settings_page.dart';
import 'package:solosoul_flutter/presentation/pages/trash_page.dart';
import 'package:solosoul_flutter/presentation/pages/search_page.dart';
import 'package:solosoul_flutter/presentation/pages/object_workspace_page.dart';
import 'package:solosoul_flutter/presentation/pages/object_editor_page.dart';
import 'package:solosoul_flutter/presentation/pages/page_editor_page.dart';
import 'package:solosoul_flutter/presentation/providers/auth_provider.dart';
import 'package:solosoul_flutter/presentation/widgets/scaffold_with_sidebar.dart';

/// Route paths matching AppRoutes constants
class AppRoutes {
  AppRoutes._();

  static const String login = '/login';
  static const String home = '/home';
  static const String profile = '/profile';
  static const String travel = '/travel';
  static const String financial = '/financial';
  static const String professional = '/professional';
  static const String settings = '/settings';
  static const String dataManagement = '/settings/data-management';
  static const String securitySettings = '/security_settings';
  static const String operationLog = '/operation_log';
  static const String sensitivitySettings = '/sensitivity_settings';
  static const String trash = '/trash';
  static const String search = '/search';
  static const String objects = '/objects';
  static const String objectEditor = '/object_editor';
  static const String pageEditor = '/page_editor';
}

/// Pages that don't require authentication
const _publicRoutes = {
  AppRoutes.login,
  '/', // splash redirects to login or home
};

/// Creates the GoRouter instance
GoRouter createRouter(WidgetRef ref) {
  return GoRouter(
    initialLocation: '/', // Must start at / to run SplashPage which initializes account manager
    debugLogDiagnostics: true,
    redirect: (context, state) {
      final authAsync = ref.read(authNotifierProvider);
      if (authAsync.isLoading) return null;
      final isUnlocked = authAsync.value == AuthState.unlocked;
      final currentPath = state.matchedLocation;

      // Public routes are always accessible
      if (_publicRoutes.contains(currentPath)) {
        // If already unlocked, redirect public routes to home
        if (isUnlocked && currentPath == AppRoutes.login) {
          return AppRoutes.home;
        }
        return null;
      }

      // All other routes require authentication
      if (!isUnlocked) {
        return AppRoutes.login;
      }

      return null;
    },
    routes: [
      GoRoute(
        path: '/',
        builder: (context, state) => const SplashPage(),
      ),
      GoRoute(
        path: AppRoutes.login,
        builder: (context, state) => const LoginPage(),
      ),
      ShellRoute(
        builder: (context, state, child) => ScaffoldWithSidebar(child: child),
        routes: [
          GoRoute(
            path: AppRoutes.home,
            builder: (context, state) => const HomePage(),
          ),
          GoRoute(
            path: AppRoutes.profile,
            builder: (context, state) => const ProfilePage(),
          ),
          GoRoute(
            path: AppRoutes.travel,
            builder: (context, state) => const TravelPage(),
          ),
          GoRoute(
            path: AppRoutes.financial,
            builder: (context, state) => const FinancialPage(),
          ),
          GoRoute(
            path: AppRoutes.professional,
            builder: (context, state) => const ProfessionalPage(),
          ),
          GoRoute(
            path: AppRoutes.settings,
            builder: (context, state) => const SettingsPage(),
          ),
          GoRoute(
            path: AppRoutes.dataManagement,
            builder: (context, state) => const DataManagementPage(),
          ),
          GoRoute(
            path: AppRoutes.securitySettings,
            builder: (context, state) => const SecuritySettingsPage(),
          ),
          GoRoute(
            path: AppRoutes.operationLog,
            builder: (context, state) => const OperationLogPage(),
          ),
          GoRoute(
            path: AppRoutes.sensitivitySettings,
            builder: (context, state) => const SensitivitySettingsPage(),
          ),
          GoRoute(
            path: AppRoutes.trash,
            builder: (context, state) => const TrashPage(),
          ),
          GoRoute(
            path: AppRoutes.search,
            builder: (context, state) => const SearchPage(),
          ),
          GoRoute(
            path: AppRoutes.objects,
            builder: (context, state) => const ObjectWorkspacePage(),
          ),
          GoRoute(
            path: '${AppRoutes.objects}/:id',
            builder: (context, state) {
              final id = state.pathParameters['id']!;
              return ObjectWorkspacePage(objectId: id);
            },
          ),
          GoRoute(
            path: AppRoutes.objectEditor,
            builder: (context, state) {
              final objectId = state.uri.queryParameters['id'];
              final parentId = state.uri.queryParameters['parentId'];
              return ObjectEditorPage(
                objectId: objectId,
                parentId: parentId,
              );
            },
          ),
          GoRoute(
            path: AppRoutes.pageEditor,
            builder: (context, state) {
              final objectId = state.uri.queryParameters['id'];
              final parentId = state.uri.queryParameters['parentId'];
              return PageEditorPage(
                objectId: objectId,
                parentId: parentId,
              );
            },
          ),
        ],
      ),
    ],
  );
}
