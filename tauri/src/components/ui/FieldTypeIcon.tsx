import { memo } from 'react';
import {
  Type,
  AlignLeft,
  Hash,
  Calendar,
  Clock,
  CheckSquare,
  List,
  ListChecks,
  Link,
  Mail,
  Phone,
  File,
} from 'lucide-react';
import type { PropertyType } from '@/types/template';

/** Map field type to a Lucide icon for visual indication.
 *  §29 — 唯一真理来源：所有字段类型图标统一由此组件提供。
 */
export const FieldTypeIcon = memo(function FieldTypeIcon({
  type,
  size = 14,
}: {
  type: PropertyType;
  size?: number;
}) {
  const style = { color: 'var(--text-tertiary)', flexShrink: 0 } as React.CSSProperties;
  switch (type) {
    case 'text':
      return <Type size={size} style={style} />;
    case 'multiline':
      return <AlignLeft size={size} style={style} />;
    case 'number':
      return <Hash size={size} style={style} />;
    case 'date':
      return <Calendar size={size} style={style} />;
    case 'datetime':
      return <Clock size={size} style={style} />;
    case 'boolean':
      return <CheckSquare size={size} style={style} />;
    case 'select':
      return <List size={size} style={style} />;
    case 'multiselect':
      return <ListChecks size={size} style={style} />;
    case 'url':
      return <Link size={size} style={style} />;
    case 'email':
      return <Mail size={size} style={style} />;
    case 'phone':
      return <Phone size={size} style={style} />;
    case 'file':
      return <File size={size} style={style} />;
    default:
      return <Type size={size} style={style} />;
  }
});
