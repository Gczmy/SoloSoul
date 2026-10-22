import { useRef, useState, type ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AndroidSheet } from './AndroidSheet';
import { AndroidChoiceList, type AndroidChoice } from './AndroidChoiceList';
import { Button } from '@/components/ui/Button';
import styles from './AndroidChoice.module.css';

type Selection =
  | { multiple?: false; value: string; onChange: (value: string) => void }
  | { multiple: true; value: string[]; onChange: (value: string[]) => void };

/** 安卓专用：单选即选即用，多选保留草稿，取消/返回不写入父表单。 */
export function AndroidChoiceField(
  props: Selection & {
    id?: string;
    title: string;
    choices: AndroidChoice[];
    placeholder?: string;
    disabled?: boolean;
    summary?: ReactNode;
  },
) {
  const { t } = useTranslation('common');
  const { id, title, choices, placeholder, disabled, summary } = props;
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<string[]>([]);
  const trigger = useRef<HTMLButtonElement>(null);
  const selected = Array.isArray(props.value) ? props.value : [props.value];
  const labels = choices.filter((choice) => selected.includes(choice.value)).map((c) => c.label);
  // 保留存量未知值的可见性，避免看似空白却仍会保存旧值。
  const unknown = selected.filter((v) => v && !choices.some((choice) => choice.value === v));
  const text = [...labels, ...unknown].join(' · ');
  const close = () => setOpen(false);
  return (
    <>
      <button
        ref={trigger}
        id={id}
        type="button"
        className={styles.trigger}
        aria-label={title}
        aria-haspopup="dialog"
        aria-expanded={open && !disabled}
        disabled={disabled}
        onClick={() => {
          setDraft(choices.filter((c) => selected.includes(c.value)).map((c) => c.value));
          setOpen(true);
        }}
      >
        <span>{summary ?? (text || placeholder || t('select_option'))}</span>
        <ChevronDown size={20} aria-hidden="true" />
      </button>
      {open && !disabled && (
        <AndroidSheet
          title={title}
          material="solid"
          className={styles.sheet}
          zIndex={6000}
          trigger={trigger.current}
          onClose={close}
        >
          <div className={styles.body} data-android-choice-scroll>
            <AndroidChoiceList
              label={title}
              choices={choices}
              value={props.multiple ? draft : props.value}
              multiple={props.multiple}
              onChoose={(value) => {
                if (props.multiple)
                  setDraft((previous) =>
                    choices
                      .filter((c) =>
                        c.value === value ? !previous.includes(value) : previous.includes(c.value),
                      )
                      .map((c) => c.value),
                  );
                else {
                  props.onChange(value);
                  close();
                }
              }}
            />
          </div>
          {props.multiple && (
            <footer className={styles.actions}>
              <Button variant="secondary" onClick={close}>
                {t('cancel')}
              </Button>
              <Button
                onClick={() => {
                  props.onChange(draft);
                  close();
                }}
              >
                {t('confirm')}
              </Button>
            </footer>
          )}
        </AndroidSheet>
      )}
    </>
  );
}
