import 'package:flutter/material.dart';
import 'package:solosoul_flutter/presentation/theme/app_theme.dart';

class DeleteAccountButton extends StatefulWidget {
  const DeleteAccountButton({super.key, required this.onTap});

  final VoidCallback onTap;

  @override
  State<DeleteAccountButton> createState() => _DeleteAccountButtonState();
}

class _DeleteAccountButtonState extends State<DeleteAccountButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(vertical: 16),
          decoration: BoxDecoration(
            border: Border.all(color: AppTheme.errorColor),
            borderRadius: BorderRadius.circular(12),
            color: _isHovered
                ? AppTheme.errorColor.withValues(alpha: 0.1)
                : Colors.transparent,
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: AppTheme.errorColor.withValues(alpha: 0.3),
                      blurRadius: 0,
                      spreadRadius: 0,
                    ),
                  ]
                : null,
          ),
          child: const Text(
            'Delete Account',
            textAlign: TextAlign.center,
            style: TextStyle(
              color: AppTheme.errorColor,
              fontSize: 16,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
  }
}
