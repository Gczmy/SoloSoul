import { create } from 'zustand';
import { useAuthStore } from './authStore';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { onVaultDataChange } from '@/lib/vaultDataChanges';
import { objectEditLocked } from '@/lib/objectEditLock';

/** Only navigation metadata is retained; this store never keeps field values. */
export interface PageIndexObject {
  id: string;
  name: string;
  sectionId: string;
  iconName: string;
  editLocked?: boolean;
}

interface PageIndexState {
  objects: PageIndexObject[];
  loading: boolean;
  failed: boolean;
  refresh: () => Promise<void>;
  clear: () => void;
}

const requests = createSessionRequests();
let observers = 0;

export const useDesktopPageIndexStore = create<PageIndexState>((set) => ({
  objects: [],
  loading: false,
  failed: false,
  refresh: async () => {
    const { currentAccount, isAuthenticated } = useAuthStore.getState();
    if (!currentAccount || !isAuthenticated) return;
    const request = requests.begin('index', currentAccount.id);
    const setCurrent = request.guardSet<PageIndexState>(set);
    setCurrent({ loading: true, failed: false });
    try {
      const objects = await request.invokeTyped('object_list', { accountId: currentAccount.id });
      setCurrent({
        objects: objects
          .filter(
            (object) =>
              !object.isDeleted && object.typeId !== 'page' && object.typeId !== 'unknown',
          )
          .map(({ id, name, sectionType, typeId, iconName, properties }) => ({
            id,
            name,
            sectionId: sectionType || typeId,
            iconName,
            ...(objectEditLocked(properties) ? { editLocked: true } : {}),
          })),
        loading: false,
      });
    } catch {
      setCurrent({ loading: false, failed: true });
    }
  },
  clear: () => {
    requests.invalidate();
    set({ objects: [], loading: false, failed: false });
  },
}));

/** Persistent shell owns this subscription, independently of workspace filters. */
export function observeDesktopPageIndex() {
  observers += 1;
  void useDesktopPageIndexStore.getState().refresh();
  return () => {
    observers -= 1;
    if (observers === 0) useDesktopPageIndexStore.getState().clear();
  };
}

onRequestSessionChange(() => useDesktopPageIndexStore.getState().clear());
onVaultDataChange(() => {
  if (observers > 0) void useDesktopPageIndexStore.getState().refresh();
});
