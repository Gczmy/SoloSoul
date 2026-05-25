import 'package:flutter/material.dart';
import 'package:solosoul_flutter/core/models/semantic_type_registry.dart';
import 'package:solosoul_flutter/gen/l10n/app_localizations.dart';

/// 语义类型选择弹窗。
///
/// 允许用户从预定义的语义类型库中选择一个类型，
/// 用于为字段绑定语义含义。
class SemanticTypePickerSheet extends StatefulWidget {
  final String? currentSemanticType;
  final String languageCode;
  final ValueChanged<String?> onSelected;

  const SemanticTypePickerSheet({
    super.key,
    this.currentSemanticType,
    required this.languageCode,
    required this.onSelected,
  });

  @override
  State<SemanticTypePickerSheet> createState() => _SemanticTypePickerSheetState();
}

class _SemanticTypePickerSheetState extends State<SemanticTypePickerSheet> {
  String _searchQuery = '';
  String? _expandedCategory;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    final types = _searchQuery.isEmpty
        ? SemanticTypeRegistry.allTypes
        : SemanticTypeRegistry.search(_searchQuery, widget.languageCode);

    // 按分类分组
    final grouped = <String, List<SemanticFieldType>>{};
    for (final type in types) {
      grouped.putIfAbsent(type.category, () => []).add(type);
    }

    // 保持分类顺序
    final orderedCategories = SemanticTypeRegistry.categories
        .where((c) => grouped.containsKey(c))
        .toList();

    return DraggableScrollableSheet(
      initialChildSize: 0.7,
      maxChildSize: 0.9,
      minChildSize: 0.4,
      expand: false,
      builder: (context, scrollController) {
        return Column(
          children: [
            // 拖动手柄
            Container(
              margin: const EdgeInsets.only(top: 8, bottom: 8),
              width: 36,
              height: 4,
              decoration: BoxDecoration(
                color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
                borderRadius: BorderRadius.circular(2),
              ),
            ),

            // 标题
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Text(
                l10n.semanticTypePickerTitle,
                style: theme.textTheme.titleLarge,
              ),
            ),

            // 搜索栏
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: TextField(
                decoration: InputDecoration(
                  hintText: l10n.semanticTypeSearchHint,
                  prefixIcon: const Icon(Icons.search),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                ),
                onChanged: (query) => setState(() => _searchQuery = query),
              ),
            ),

            // 分类列表
            Expanded(
              child: ListView.builder(
                controller: scrollController,
                itemCount: orderedCategories.length + 1, // +1 for "None" option
                itemBuilder: (context, index) {
                  if (index == orderedCategories.length) {
                    return _buildNoneOption(l10n, theme);
                  }

                  final category = orderedCategories[index];
                  final categoryTypes = grouped[category]!;
                  final isExpanded = _expandedCategory == category || _searchQuery.isNotEmpty;

                  return _buildCategoryTile(
                    category,
                    categoryTypes,
                    isExpanded,
                    l10n,
                    theme,
                  );
                },
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildCategoryTile(
    String category,
    List<SemanticFieldType> types,
    bool isExpanded,
    AppLocalizations l10n,
    ThemeData theme,
  ) {
    final categoryLabel = SemanticTypeRegistry.getCategoryLabel(category, widget.languageCode);
    final categoryIcon = SemanticTypeRegistry.getCategoryIcon(category);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 分类标题（可点击展开/折叠）
        InkWell(
          onTap: _searchQuery.isNotEmpty
              ? null
              : () => setState(() {
                    _expandedCategory = _expandedCategory == category ? null : category;
                  }),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            child: Row(
              children: [
                Icon(categoryIcon, size: 20, color: theme.colorScheme.primary),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    categoryLabel,
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                if (_searchQuery.isEmpty)
                  Icon(
                    isExpanded ? Icons.expand_less : Icons.expand_more,
                    size: 20,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
              ],
            ),
          ),
        ),

        // 分类下的语义类型列表
        if (isExpanded)
          ...types.map((type) => _buildTypeTile(type, l10n, theme)),
      ],
    );
  }

  Widget _buildTypeTile(SemanticFieldType type, AppLocalizations l10n, ThemeData theme) {
    final isSelected = type.id == widget.currentSemanticType;
    final label = type.getLabel(widget.languageCode);
    final description = type.getDescription(widget.languageCode);

    return InkWell(
      onTap: () {
        widget.onSelected(type.id);
        Navigator.of(context).pop();
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
        decoration: BoxDecoration(
          color: isSelected
              ? theme.colorScheme.primaryContainer.withValues(alpha: 0.5)
              : null,
          borderRadius: BorderRadius.circular(8),
          border: isSelected
              ? Border.all(color: theme.colorScheme.primary, width: 1)
              : null,
        ),
        child: Row(
          children: [
            Icon(
              type.icon,
              size: 20,
              color: isSelected
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: isSelected ? FontWeight.w600 : FontWeight.normal,
                      color: isSelected
                          ? theme.colorScheme.primary
                          : theme.colorScheme.onSurface,
                    ),
                  ),
                  if (description.isNotEmpty)
                    Text(
                      description,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                ],
              ),
            ),
            if (isSelected)
              Icon(
                Icons.check_circle,
                size: 20,
                color: theme.colorScheme.primary,
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildNoneOption(AppLocalizations l10n, ThemeData theme) {
    final isSelected = widget.currentSemanticType == null;

    return InkWell(
      onTap: () {
        widget.onSelected(null);
        Navigator.of(context).pop();
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: isSelected
              ? theme.colorScheme.surfaceContainerHighest
              : null,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: theme.colorScheme.outline.withValues(alpha: 0.3),
          ),
        ),
        child: Row(
          children: [
            Icon(
              Icons.label_off,
              size: 20,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                l10n.semanticTypeNone,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            if (isSelected)
              Icon(
                Icons.check_circle,
                size: 20,
                color: theme.colorScheme.primary,
              ),
          ],
        ),
      ),
    );
  }
}
