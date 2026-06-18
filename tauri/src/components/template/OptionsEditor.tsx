import { useState } from 'react';

interface OptionsEditorProps {
  options: string[];
  onChange: (opts: string[]) => void;
  fieldName: string;
  fieldType: 'select' | 'multiselect';
}

export function OptionsEditor({ options, onChange, fieldName, fieldType }: OptionsEditorProps) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState('');

  const handleOpen = () => {
    setEditing(options.join('\n'));
    setOpen(true);
  };

  return (
    <>
      <button
        type="button"
        onClick={handleOpen}
        title="编辑选项"
        style={{
          height: 36,
          padding: '0 10px',
          borderRadius: 6,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-elevated)',
          color: 'var(--text-secondary)',
          fontSize: 13,
          cursor: 'pointer',
          whiteSpace: 'nowrap',
          lineHeight: '36px',
        }}
      >
        {options.length > 0 ? `${options.length} 个选项` : '添加选项'}
      </button>
      {open && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 99999,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.35)',
            backdropFilter: 'blur(4px)',
          }}
          onClick={() => setOpen(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 16,
              padding: '28px 32px',
              maxWidth: 420,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            <h3 style={{ margin: '0 0 4px', fontSize: 16, fontWeight: 600 }}>
              {fieldType === 'multiselect' ? '编辑多选选项' : '编辑单选选项'}
              <span
                style={{
                  fontWeight: 400,
                  color: 'var(--text-secondary)',
                  marginLeft: 8,
                  fontSize: 14,
                }}
              >
                {fieldName}
              </span>
            </h3>
            <p style={{ margin: '0 0 16px', fontSize: 12, color: 'var(--text-tertiary)' }}>
              {fieldType === 'multiselect'
                ? '每行输入一个选项，可多选'
                : '每行输入一个选项，只能选一项'}
            </p>
            <textarea
              value={editing}
              onChange={(e) => setEditing(e.target.value)}
              rows={8}
              style={{
                width: '100%',
                padding: '10px 12px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 14,
                fontFamily: 'inherit',
                resize: 'vertical',
                boxSizing: 'border-box',
                outline: 'none',
              }}
              autoFocus
            />
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
              <button
                type="button"
                onClick={() => setOpen(false)}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: 14,
                  color: 'var(--text-secondary)',
                }}
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => {
                  const opts = editing
                    .split('\n')
                    .map((s: string) => s.trim())
                    .filter(Boolean);
                  onChange(opts);
                  setOpen(false);
                }}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: 'none',
                  background: 'var(--accent-primary)',
                  cursor: 'pointer',
                  fontSize: 14,
                  color: 'white',
                }}
              >
                确定
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
