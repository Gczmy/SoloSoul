import { useTranslation } from 'react-i18next';
import {
  Type,
  AlignLeft,
  Hash,
  Calendar,
  Clock,
  Link,
  Mail,
  Phone,
  Paperclip,
  CheckSquare,
  ChevronDown,
  List,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';

interface PreviewField {
  id: string;
  nameFallback: string;
  type: string;
  sensitivityLevel?: string;
  required?: boolean;
}

interface TemplatePreviewProps {
  templateName: string;
  category: string;
  fields: PreviewField[];
}

const typeIconMap: Record<string, React.ElementType> = {
  text: Type,
  multiline: AlignLeft,
  number: Hash,
  date: Calendar,
  datetime: Clock,
  url: Link,
  email: Mail,
  phone: Phone,
  file: Paperclip,
  boolean: CheckSquare,
  select: ChevronDown,
  multiselect: List,
};

export function TemplatePreview({ templateName, category, fields }: TemplatePreviewProps) {
  const { t } = useTranslation(['editor', 'navigation']);

  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
        <span style={{ fontSize: 13, fontWeight: 600 }}>{templateName}</span>
        <span
          style={{
            fontSize: 10,
            padding: '2px 6px',
            borderRadius: 4,
            background: 'rgba(91,124,153,0.08)',
            color: 'var(--text-secondary)',
            textTransform: 'uppercase',
          }}
        >
          {t(`navigation:${category}`, category)}
        </span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {fields.map((field) => {
          const Icon = typeIconMap[field.type];
          return (
            <div
              key={field.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '6px 8px',
                borderRadius: 6,
                background: 'var(--bg-subtle)',
              }}
            >
              <span
                style={{
                  width: 20,
                  height: 20,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: 4,
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-secondary)',
                }}
                title={field.type}
              >
                {Icon ? <Icon size={12} strokeWidth={2} /> : <span style={{ fontSize: 10 }}>?</span>}
              </span>
              <span style={{ fontSize: 12, flex: 1, color: 'var(--text-primary)' }}>
                {t(`editor:fields.${field.id}`, field.nameFallback)}
              </span>
              <div style={{ display: 'flex', gap: 4 }}>
                {field.required && (
                  <span
                    style={{
                      fontSize: 9,
                      padding: '1px 4px',
                      borderRadius: 3,
                      background: 'rgba(239,68,68,0.1)',
                      color: '#ef4444',
                    }}
                  >
                    {t('editor:required')}
                  </span>
                )}
                {field.sensitivityLevel && field.sensitivityLevel !== 'public' && (
                  <span
                    style={{
                      fontSize: 9,
                      padding: '1px 4px',
                      borderRadius: 3,
                      background:
                        field.sensitivityLevel === 'critical'
                          ? 'rgba(239,68,68,0.1)'
                          : field.sensitivityLevel === 'sensitive'
                            ? 'rgba(234,179,8,0.1)'
                            : 'rgba(107,114,128,0.1)',
                      color:
                        field.sensitivityLevel === 'critical'
                          ? '#ef4444'
                          : field.sensitivityLevel === 'sensitive'
                            ? '#eab308'
                            : '#6b7280',
                    }}
                  >
                    {t(`editor:sensitivity_levels.${field.sensitivityLevel}`, field.sensitivityLevel)}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </Card>
  );
}
